mod character;
mod character_screen_connection;
mod game_data;
mod ipc_connection;
mod login_character;
mod packet_handler;
mod server;
mod creature;
mod item;

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
};

use concurrent_queue::ConcurrentQueue;
use gameserver_migration::{Migrator, MigratorTrait};
use interprocess::local_socket::tokio::SendHalf;
use ipc_comms::{
    SessionKeyResponse,
    realm_types::{RealmCategory, RealmType},
};
use log::{error, info, warn};
use packets::account_result::AccountResult;
use sea_orm::{ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, tcp::OwnedReadHalf},
    sync::Mutex,
};

use crate::{
    character::Character,
    character_screen_connection::{CharacterScreenConnection, CharacterScreenResult},
    game_data::GameDataAccessor,
    ipc_connection::start_ipc_task,
    server::Server,
};

#[derive(Deserialize, Serialize)]
struct AppSettings {
    bind_to: SocketAddr,
    database_path: PathBuf,

    server_id: u8,
    server_type: RealmType,
    // Sent to clients to show server name
    server_name: String,
    // Sent to clients to connect to
    server_address: SocketAddr,
    server_category: RealmCategory,

    // This is a name for a local socket that is used to create a communication
    // tunnel between gameservers and an authserver
    ipc_socket_name: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            bind_to: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8085)),
            database_path: PathBuf::from_str("gameserver.db").unwrap(),
            server_id: 0,
            server_type: RealmType::Normal,
            server_name: "Change me!".to_owned(),
            server_address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 8085)),
            server_category: RealmCategory::Unkn,

            ipc_socket_name: "wow_server.sock".to_owned(),
        }
    }
}

const TICKRATE: u32 = 20; // Ticks per second

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let config: AppSettings = confy::load_path("./gameserver_config.toml").unwrap();

    let db: DatabaseConnection = Database::connect(format!(
        "sqlite://{}?mode=rwc",
        config.database_path.display()
    ))
    .await
    .unwrap();
    Migrator::up(&db, None).await.unwrap();

    let server_pipe: Arc<Mutex<Option<SendHalf>>> = Arc::new(Mutex::new(None));
    // Once connection is established, the key gets removed from here
    let player_session_keys: Arc<Mutex<HashMap<String, SessionKeyResponse>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let exiting = Arc::new(AtomicBool::new(false));
    start_ipc_task(
        config.ipc_socket_name,
        db.clone(),
        exiting.clone(),
        server_pipe.clone(),
        player_session_keys.clone(),
        config.server_id,
        config.server_type,
        0, //TODO: real flags values
        config.server_name,
        config.server_address,
        config.server_category,
    );

    let world_transition_character_queue: Arc<ConcurrentQueue<(Character, OwnedReadHalf)>> =
        Arc::new(ConcurrentQueue::unbounded());

    let game_data_accessor = GameDataAccessor::new(db.clone());
    {
        let world_transition_character_queue = world_transition_character_queue.clone();

        let game_data_accessor = game_data_accessor.clone();
        tokio::spawn(async move {
            let socket = TcpListener::bind(config.bind_to)
                .await
                .expect("Failed to bind socket");

            loop {
                let (stream, ip) = match socket.accept().await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Failed to accept connection: {}", e);
                        continue;
                    }
                };
                info!("New connection from: {}", ip);

                let player_session_keys = player_session_keys.clone();
                let server_pipe = server_pipe.clone();
                let game_data_accessor = game_data_accessor.clone();
                let db = db.clone();
                let world_transition_character_queue = world_transition_character_queue.clone();
                tokio::spawn(async move {
                    let mut conn = CharacterScreenConnection::authenticate(
                        stream,
                        player_session_keys,
                        server_pipe,
                        db.clone(),
                        game_data_accessor,
                    )
                    .await
                    .unwrap();

                    loop {
                        match conn.connection_loop().await {
                            CharacterScreenResult::WorldTransition { guid } => {
                                let model = match gameserver_entity::character::Entity::find_by_id(
                                    guid.get_u32().get() as i32,
                                )
                                .filter(
                                    gameserver_entity::character::Column::AccountId
                                        .eq(conn.account_id),
                                )
                                .one(&db)
                                .await
                                {
                                    Ok(Some(v)) => v,
                                    Ok(None) => {
                                        let response = packets::server::SMSG_CHAR_LOGIN_FAILED {
                                            result: AccountResult::CHAR_LOGIN_NO_CHARACTER,
                                        };

                                        warn!(
                                            "Client (account_id: {}) tried to log into a character that doesn't exist or not owned by the client (guid: {})",
                                            conn.account_id,
                                            guid.get_u32()
                                        );

                                        if let Err(e) = conn
                                            .stream
                                            .write_all(&response.to_bytes(Some(conn.session_key)))
                                            .await
                                        {
                                            error!(
                                                "Failed to send SMSG_CHAR_LOGIN_FAILED to client (account_id: {}). Error: {:?}",
                                                conn.account_id, e
                                            )
                                        };
                                        continue;
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to get client's character due to a DB error (account_id: {}, character_id: {}). Error: {}",
                                            conn.account_id,
                                            guid.get_u32(),
                                            e
                                        );
                                        let response = packets::server::SMSG_CHAR_DELETE {
                                            result: AccountResult::CHAR_LOGIN_FAILED,
                                        };

                                        if let Err(e) = conn
                                            .stream
                                            .write_all(&response.to_bytes(Some(conn.session_key)))
                                            .await
                                        {
                                            error!(
                                                "Failed to send SMSG_CHAR_DELETE to client (account_id: {}). Error: {:?}",
                                                conn.account_id, e
                                            )
                                        };
                                        continue;
                                    }
                                };

                                let (rx, tx) = conn.stream.into_split();

                                let mut character =
                                    Character::from_model(tx, conn.session_key, model);

                                character.player_fields.main_backpack_slots[0] =
                                    Some(common::guid::Guid::from_u32(
                                        std::num::NonZeroU32::new(2).unwrap(),
                                    ))
                                    .into();

                                info!(
                                    "Transitioning client's character (client_id: {}, character_id: {}) into a game world",
                                    character.account_id, 0
                                ); //TODO: character_id

                                // If this fails, the client will be disconnected anyway due to Drop being called
                                let _ = world_transition_character_queue.push((character, rx));
                                return;
                            }
                            CharacterScreenResult::ClientDisconnect => {
                                return;
                            }
                        }
                    }
                });
            }
        });
    }

    let max_sleep_for_ms = (1000 / TICKRATE) as i64;

    let mut server = Server::new(world_transition_character_queue, game_data_accessor);
    loop {
        let new_game_time = chrono::Local::now();
        let diff = (new_game_time - server.game_time).abs();
        server.game_time = new_game_time;

        server.update(diff).await;

        let update_took = (chrono::Local::now() - server.game_time).num_milliseconds();
        let left_to_sleep = max_sleep_for_ms - update_took;
        if left_to_sleep > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(left_to_sleep as u64)).await;
        }
    }
}

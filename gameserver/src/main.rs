mod character_selection_screen;
mod creature_spawner;
mod dirty_tracking_map;
mod game_data;
mod guid_allocator;
mod ipc_connection;
mod objects;
mod packet_handler;
mod server;
mod sparse_set;

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

use concurrent_queue::ConcurrentQueue;
use interprocess::local_socket::tokio::SendHalf;
use ipc_comms::{
    SessionKeyResponse,
    realm_types::{RealmCategory, RealmType},
};
use log::{error, info, warn};
use packets::{account_result::AccountResult, server::ServerPacket};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, tcp::OwnedReadHalf},
    sync::{Mutex, mpsc},
};

use crate::{
    character_selection_screen::character_screen_connection::{
        CharacterScreenConnection, CharacterScreenResult,
    },
    game_data::GameDataAccessor,
    ipc_connection::start_ipc_task,
    objects::character::Character,
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
const TICK_DURATION: Duration = Duration::from_millis(1000 / TICKRATE as u64);
const LAG_WARNING_INTERVAL: Duration = Duration::from_secs(1);

const CONFIG_FILE: &str = "gameserver_config.toml";

#[cfg(debug_assertions)]
fn locate_working_dir() {
    if std::path::Path::new(CONFIG_FILE).exists() {
        return;
    }
    let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    if crate_dir.join(CONFIG_FILE).exists() {
        std::env::set_current_dir(crate_dir).unwrap();
    }
}

#[tokio::main]
async fn main() {
    #[cfg(debug_assertions)]
    locate_working_dir();

    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let config: AppSettings = confy::load_path(CONFIG_FILE).unwrap();

    let db = SqlitePoolOptions::new()
        .connect(&format!(
            "sqlite://{}?mode=rwc",
            config.database_path.display()
        ))
        .await
        .unwrap();
    sqlx::migrate!().run(&db).await.unwrap();

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

    let world_transition_character_queue: Arc<
        ConcurrentQueue<(Box<Character>, OwnedReadHalf, (usize, u8))>,
    > = Arc::new(ConcurrentQueue::unbounded());

    let game_data_accessor = GameDataAccessor::new(db.clone());

    let (character_transition_to_character_screen_tx, character_transition_to_character_screen_rx) =
        mpsc::unbounded_channel::<(Character, OwnedReadHalf, (usize, u8))>();

    {
        let world_transition_character_queue = world_transition_character_queue.clone();

        let game_data_accessor = game_data_accessor.clone();
        let db = db.clone();
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
                    let conn = CharacterScreenConnection::authenticate(
                        stream,
                        player_session_keys,
                        server_pipe,
                        db.clone(),
                        game_data_accessor.clone(),
                    )
                    .await
                    .unwrap();

                    run_character_selection_screen_loop(
                        conn,
                        &game_data_accessor,
                        &db,
                        &world_transition_character_queue,
                    )
                    .await;
                });
            }
        });
    }

    {
        let mut character_transition_to_character_screen_rx =
            character_transition_to_character_screen_rx;
        let game_data_accessor = game_data_accessor.clone();
        let world_transition_character_queue = world_transition_character_queue.clone();
        let db = db.clone();
        tokio::spawn(async move {
            loop {
                let Some((character, read_half, decrypt_data)) =
                    character_transition_to_character_screen_rx.recv().await
                else {
                    return;
                };
                info!(
                    "Transitioning client's character (client_id: {}, character_id: {}) into a character selection screen",
                    character.account_id, 0
                ); //TODO: character_id

                let db = db.clone();
                let game_data_accessor = game_data_accessor.clone();
                let world_transition_character_queue = world_transition_character_queue.clone();
                tokio::spawn(async move {
                    let conn = CharacterScreenConnection::from_game_character(
                        character,
                        read_half,
                        decrypt_data,
                        db.clone(),
                        game_data_accessor.clone(),
                    );
                    run_character_selection_screen_loop(
                        conn,
                        &game_data_accessor,
                        &db,
                        &world_transition_character_queue,
                    )
                    .await;
                });
            }
        });
    }

    let mut server = Server::new(
        world_transition_character_queue,
        game_data_accessor,
        character_transition_to_character_screen_tx,
        db,
    )
    .await
    .expect("failed to fetch all required data from database");
    let mut last_lag_warning = None;
    loop {
        let new_game_time = chrono::Local::now();
        let diff = (new_game_time - server.game_time).abs();
        server.game_time = new_game_time;

        let update_started = Instant::now();
        server.update(diff).await;
        let update_took = update_started.elapsed();

        if update_took > TICK_DURATION {
            if last_lag_warning
                .is_none_or(|last_warning: Instant| last_warning.elapsed() >= LAG_WARNING_INTERVAL)
            {
                let behind = update_took - TICK_DURATION;
                last_lag_warning = Some(Instant::now());
                warn!(
                    "Can't keep up! Game tick took {:.2} ms, {:.2} ms ({:.2} ticks) behind",
                    update_took.as_secs_f64() * 1000.0,
                    behind.as_secs_f64() * 1000.0,
                    behind.as_secs_f64() / TICK_DURATION.as_secs_f64(),
                );
            }

            tokio::task::yield_now().await;
        } else {
            tokio::time::sleep(TICK_DURATION - update_took).await;
        }
    }
}

async fn run_character_selection_screen_loop(
    mut conn: CharacterScreenConnection,
    game_data_accessor: &GameDataAccessor,
    db: &Pool<Sqlite>,
    world_transition_character_queue: &ConcurrentQueue<(
        Box<Character>,
        OwnedReadHalf,
        (usize, u8),
    )>,
) {
    loop {
        match conn.connection_loop().await {
            CharacterScreenResult::WorldTransition { guid } => {
                let _ = conn.stream.set_nodelay(true);

                let (rx, tx) = conn.stream.into_split();

                let character = match Character::load_from_db(
                    game_data_accessor,
                    db,
                    guid,
                    conn.account_id,
                    tx,
                    conn.session_key,
                    conn.encrypt_data,
                )
                .await
                {
                    Ok(v) => v,
                    Err((mut tx, sqlx::Error::RowNotFound)) => {
                        let response = packets::server::SMSG_CHAR_LOGIN_FAILED {
                            result: AccountResult::CHAR_LOGIN_NO_CHARACTER,
                        };

                        warn!(
                            "Client (account_id: {}) tried to log into a character that doesn't exist or not owned by the client (guid: {})",
                            conn.account_id,
                            guid.get_u32()
                        );

                        if let Err(e) = tx
                            .write_all(
                                &response.to_bytes(Some(conn.session_key), &mut conn.encrypt_data),
                            )
                            .await
                        {
                            error!(
                                "Failed to send SMSG_CHAR_LOGIN_FAILED to client (account_id: {}). Error: {:?}",
                                conn.account_id, e
                            )
                        };
                        conn.stream = tx.reunite(rx).unwrap();
                        continue;
                    }
                    Err((mut tx, e)) => {
                        error!(
                            "Failed to get client's character due to a DB error (account_id: {}, character_id: {}). Error: {}",
                            conn.account_id,
                            guid.get_u32(),
                            e
                        );
                        let response = packets::server::SMSG_CHAR_DELETE {
                            result: AccountResult::CHAR_LOGIN_FAILED,
                        };

                        if let Err(e) = tx
                            .write_all(
                                &response.to_bytes(Some(conn.session_key), &mut conn.encrypt_data),
                            )
                            .await
                        {
                            error!(
                                "Failed to send SMSG_CHAR_DELETE to client (account_id: {}). Error: {:?}",
                                conn.account_id, e
                            )
                        };
                        conn.stream = tx.reunite(rx).unwrap();
                        continue;
                    }
                };
                let character = Box::new(character);

                info!(
                    "Transitioning client's character (client_id: {}, character_id: {}) into a game world",
                    character.account_id, 0
                ); //TODO: character_id

                // If this fails, the client will be disconnected anyway due to Drop being called
                let _ = world_transition_character_queue.push((character, rx, conn.decrypt_data));
                return;
            }
            CharacterScreenResult::ClientDisconnect => {
                return;
            }
        }
    }
}

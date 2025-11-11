mod character_screen_connection;
mod game_data;
mod ipc_connection;
mod character;

use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
};

use gameserver_migration::{Migrator, MigratorTrait};
use interprocess::local_socket::tokio::SendHalf;
use ipc_comms::{
    SessionKeyResponse,
    realm_types::{RealmCategory, RealmType},
};
use log::{error, info};
use sea_orm::{Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::Mutex};

use crate::{
    character_screen_connection::{CharacterScreenConnection, CharacterScreenResult},
    game_data::GameDataAccessor,
    ipc_connection::start_ipc_task,
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

    let game_data_accessor = GameDataAccessor::new(db.clone());

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
            tokio::spawn(async move {
                let mut conn = CharacterScreenConnection::authenticate(
                    stream,
                    player_session_keys,
                    server_pipe,
                    db,
                    game_data_accessor,
                )
                .await
                .unwrap();
                match conn.connection_loop().await {
                    CharacterScreenResult::WorldTransition => todo!(),
                    CharacterScreenResult::ClientDisconnect => {
                        return;
                    }
                }
                //let player = PlayerConnection::handle_auth(stream, db).await.unwrap();
                //player.connection_loop().await;
            });
        }
    });

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

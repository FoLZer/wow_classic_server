mod auth;
mod ipc_connection;

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
};

use gameserver_migration::{Migrator, MigratorTrait};
use ipc_comms::realm_types::{RealmCategory, RealmType};
use log::{error, info};
use sea_orm::{Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

use crate::ipc_connection::start_ipc_task;

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

    let exiting = Arc::new(AtomicBool::new(false));
    start_ipc_task(
        config.ipc_socket_name,
        exiting.clone(),
        config.server_id,
        config.server_type,
        0, //TODO: real flags values
        config.server_name,
        config.server_address,
        config.server_category,
    );

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

            tokio::spawn(async move {
                //let player = PlayerConnection::handle_auth(stream, db).await.unwrap();
                //player.connection_loop().await;
            });
        }
    });

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

#![feature(iter_array_chunks)]

mod cache;
mod connection;
mod ipc_connection;
mod packets;
mod srp;

use std::{
    collections::HashMap,
    fmt::Display,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::PathBuf,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use authserver_migration::{Migrator, MigratorTrait};
use inquire::{Confirm, Password, Text};
use inquire_derive::Selectable;
use interprocess::local_socket::tokio::SendHalf;
use lazy_static::lazy_static;
use log::{error, info};
use num_bigint::BigInt;
use rand::{Rng, rngs::StdRng};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Database, DatabaseConnection, EntityTrait,
    ModelTrait, QueryFilter,
};
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    sync::{Mutex, RwLock},
};

use crate::{
    cache::CacheData,
    connection::handle_connection,
    ipc_connection::{RealmData, start_ipc_task},
};

#[derive(Deserialize, Serialize)]
struct AppSettings {
    bind_to: SocketAddr,
    database_path: PathBuf,

    // This is a name for a local socket that is used to create a communication
    // tunnel between gameservers and an authserver
    ipc_socket_name: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            bind_to: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 3724)),
            database_path: PathBuf::from_str("authserver.db").unwrap(),
            ipc_socket_name: "wow_server.sock".to_owned(),
        }
    }
}

lazy_static! {
    static ref SECURE_RNG: Mutex<StdRng> = Mutex::new(rand::make_rng());
}

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let config: AppSettings = confy::load_path("./authserver_config.toml").unwrap();

    let db: DatabaseConnection = Database::connect(format!(
        "sqlite://{}?mode=rwc",
        config.database_path.display()
    ))
    .await
    .unwrap();
    Migrator::up(&db, None).await.unwrap();

    let server_pipes: Arc<RwLock<HashMap<u8, Arc<Mutex<SendHalf>>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let active_realms: Arc<RwLock<HashMap<u8, RealmData>>> = Arc::new(RwLock::new(HashMap::new()));
    let active_sessions: Arc<RwLock<HashMap<String, (u32, [u8; 40])>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let num_player_characters_cache: Arc<RwLock<HashMap<u8, HashMap<u32, CacheData<u8, 60>>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let exiting = Arc::new(AtomicBool::new(false));

    start_ipc_task(
        &config.ipc_socket_name,
        //server_pipes.clone(),
        active_sessions.clone(),
        active_realms.clone(),
        num_player_characters_cache.clone(),
        server_pipes.clone(),
        exiting.clone(),
    )
    .await;

    {
        let db = db.clone();
        tokio::spawn(async move {
            let server_private_key = {
                let mut v = [0; 32];
                SECURE_RNG.lock().await.fill_bytes(&mut v);
                BigInt::from_bytes_le(num_bigint::Sign::Plus, &v)
            };

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

                let server_private_key = server_private_key.clone();
                let db = db.clone();
                let active_sessions = active_sessions.clone();
                let active_realms = active_realms.clone();
                let num_player_characters_cache = num_player_characters_cache.clone();
                let server_pipes = server_pipes.clone();
                tokio::spawn(async move {
                    handle_connection(
                        stream,
                        server_private_key,
                        db,
                        active_sessions,
                        active_realms,
                        num_player_characters_cache,
                        server_pipes,
                    )
                    .await;
                });
            }
        });
    }

    while !exiting.load(Ordering::Relaxed) {
        let selection = Command::select("Choose a command to run:")
            .prompt()
            .unwrap();

        match selection {
            Command::AddUser => {
                let username = Text::new("Username:").prompt().unwrap();
                let username = username.to_uppercase();
                let password = Password::new("Password:")
                    .without_confirmation()
                    .prompt()
                    .unwrap();
                let password = password.to_uppercase();
                match add_user(&db, username.clone(), &password).await {
                    Ok(_) => {
                        println!("User {username} was successfully added!");
                    }
                    Err(e) => {
                        error!("Failed to add a user: {e}");
                        println!("Failed to add a user: {e}");
                    }
                };
            }
            Command::RemoveUser => {
                let username = Text::new("Username:").prompt().unwrap();
                let username = username.to_uppercase();
                match remove_user(&db, username.clone()).await {
                    Ok(true) => {
                        println!("User {username} was successfully removed!");
                    }
                    Ok(false) => {
                        println!("User {username} was not found!");
                    }
                    Err(e) => {
                        error!("Failed to remove a user: {e}");
                        println!("Failed to remove a user: {e}");
                    }
                };
            }
            Command::Exit => {
                let ans = Confirm::new("Are you sure you want to exit?")
                    .with_default(false)
                    .prompt()
                    .unwrap();
                if ans {
                    exiting.store(true, Ordering::Relaxed);
                }
            }
        }
    }

    //TODO: wait for running tasks to exit
}

#[derive(Clone, Copy, Debug, Selectable)]
enum Command {
    AddUser,
    RemoveUser,
    Exit,
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddUser => write!(f, "Add a user"),
            Self::RemoveUser => write!(f, "Remove a user"),
            Self::Exit => write!(f, "Exit"),
        }
    }
}

async fn add_user(
    db: &DatabaseConnection,
    username: String,
    password: &str,
) -> Result<(), sea_orm::DbErr> {
    let mut salt = [0; 32];
    SECURE_RNG.lock().await.fill_bytes(&mut salt);
    let password_verifier = srp::calculate_password_verifier(&username, password, salt);
    authserver_entity::user::ActiveModel {
        id: ActiveValue::NotSet,
        account_name: ActiveValue::Set(username),
        password_verifier: ActiveValue::Set(password_verifier.to_vec()),
        salt: ActiveValue::Set(salt.to_vec()),
    }
    .insert(db)
    .await?;
    Ok(())
}

async fn remove_user(db: &DatabaseConnection, username: String) -> Result<bool, sea_orm::DbErr> {
    let Some(ent) = authserver_entity::user::Entity::find()
        .filter(authserver_entity::user::Column::AccountName.eq(username))
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    ent.delete(db).await?;

    Ok(true)
}

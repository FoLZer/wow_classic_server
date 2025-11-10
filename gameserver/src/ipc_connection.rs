use std::{
    collections::HashMap,
    io::ErrorKind,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use interprocess::local_socket::{
    GenericNamespaced,
    tokio::{SendHalf, Stream, prelude::*},
};
use ipc_comms::{
    AuthServerIpcMessage, GameServerIpcMessage, IpcError, SessionKeyResponse,
    realm_types::{RealmCategory, RealmType},
};
use log::{error, info, warn};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use tokio::sync::Mutex;

pub fn start_ipc_task(
    ipc_socket_name: String,
    db: DatabaseConnection,
    exiting: Arc<AtomicBool>,
    server_pipe: Arc<Mutex<Option<SendHalf>>>,
    player_session_keys: Arc<Mutex<HashMap<String, SessionKeyResponse>>>,
    realm_id: u8,
    realm_type: RealmType,
    flags: u8,
    realm_name: String,
    address_port: SocketAddr,
    realm_category: RealmCategory,
) {
    tokio::spawn(async move {
        let conn;
        loop {
            conn = match Stream::connect(
                ipc_socket_name
                    .as_str()
                    .to_ns_name::<GenericNamespaced>()
                    .unwrap(),
            )
            .await
            {
                Ok(v) => v,
                Err(e) if e.kind() == ErrorKind::ConnectionRefused => {
                    warn!(
                        "Failed to start an IPC stream, check if authserver is running. Retrying in 3 seconds"
                    );
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    continue;
                }
                Err(e) => {
                    panic!("{:?}", e);
                }
            };
            break;
        }

        let (mut rx, mut tx) = conn.split();

        // Send information that an auth server requires to send to the clients immediately
        let packet = AuthServerIpcMessage::RealmInfo {
            realm_id,
            realm_type: realm_type.clone(),
            flags,
            realm_name: realm_name.clone(),
            address_port,
            population: 100.0, //TODO: real population values
            realm_category: realm_category.clone(),
        };
        match packet.write(&mut tx).await {
            Ok(()) => (),
            Err(e) => {
                error!("Failed to write an IPC message to an auth server: {e:?}");
            }
        };

        {
            let mut lock = server_pipe.lock().await;
            lock.replace(tx);
        }

        while !exiting.load(Ordering::Relaxed) {
            let packet = match GameServerIpcMessage::read(&mut rx).await {
                Ok(v) => v,
                Err(IpcError::Io(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                    warn!("Auth server connection died, restarting the IPC task");
                    start_ipc_task(
                        ipc_socket_name,
                        db,
                        exiting,
                        server_pipe,
                        player_session_keys,
                        realm_id,
                        realm_type,
                        flags,
                        realm_name,
                        address_port,
                        realm_category,
                    );
                    return;
                }
                Err(e) => {
                    error!("Failed to read an IPC message from an auth server: {e:?}");
                    continue;
                }
            };

            match packet {
                GameServerIpcMessage::PlayerSessionKeyResponse {
                    account_name,
                    session_key,
                } => {
                    let mut lock = player_session_keys.lock().await;
                    lock.insert(account_name, session_key);
                }
                GameServerIpcMessage::PlayerNumCharactersRequest { account_id } => {
                    let num_characters = gameserver_entity::character::Entity::find()
                        .filter(gameserver_entity::character::Column::AccountId.eq(account_id))
                        .count(&db)
                        .await
                        .unwrap();

                    let Some(ref mut tx_lock) = *server_pipe.lock().await else {
                        panic!("IPC tx pipe was removed from outside the responsible method"); // This should never happen
                    };
                    let packet = AuthServerIpcMessage::PlayerNumCharactersResponse {
                        account_id,
                        num_characters: num_characters.min(u8::MAX as u64) as u8,
                    };
                    match packet.write(&mut *tx_lock).await {
                        Ok(()) => (),
                        Err(e) => {
                            error!("Failed to write an IPC message to an auth server: {e:?}");
                        }
                    };
                }
                GameServerIpcMessage::AuthServerError(auth_server_error) => todo!(),
                GameServerIpcMessage::AuthServerClosed => {
                    *server_pipe.lock().await = None;
                    //restart the IPC
                    start_ipc_task(
                        ipc_socket_name,
                        db,
                        exiting,
                        server_pipe,
                        player_session_keys,
                        realm_id,
                        realm_type,
                        flags,
                        realm_name,
                        address_port,
                        realm_category,
                    );
                    return;
                }
            }
        }

        {
            let Some(ref mut tx_lock) = server_pipe.lock().await.take() else {
                panic!("IPC tx pipe was removed from outside the responsible method"); // This should never happen
            };
            let packet = AuthServerIpcMessage::GameServerClosed;
            match packet.write(&mut *tx_lock).await {
                Ok(()) => (),
                Err(e) => {
                    error!("Failed to write an IPC message to an auth server: {e:?}");
                }
            };
        }
    });
}

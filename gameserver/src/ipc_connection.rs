use std::{
    io::ErrorKind, net::SocketAddr, sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    }, time::Duration
};

use interprocess::local_socket::{
    GenericNamespaced,
    tokio::{Stream, prelude::*},
};
use ipc_comms::{AuthServerIpcMessage, GameServerIpcMessage, realm_types::{RealmCategory, RealmType}};
use log::{error, info};
use tokio::sync::Mutex;

pub fn start_ipc_task(ipc_socket_name: String, exiting: Arc<AtomicBool>, realm_id: u8, realm_type: RealmType, flags: u8, realm_name: String, address_port: SocketAddr, realm_category: RealmCategory) {
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
                    info!(
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

        let tx = Arc::new(Mutex::new(tx));

        while !exiting.load(Ordering::Relaxed) {
            let packet = match GameServerIpcMessage::read(&mut rx).await {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to read an IPC message from an auth server: {e:?}");
                    continue;
                }
            };

            match packet {
                GameServerIpcMessage::PlayerSessionKeyResponse {
                    account_name,
                    session_key,
                } => todo!(),
                GameServerIpcMessage::PlayerNumCharactersRequest { account_name } => todo!(),
                GameServerIpcMessage::AuthServerError(auth_server_error) => todo!(),
                GameServerIpcMessage::AuthServerClosed => {
                    //restart the IPC
                    start_ipc_task(ipc_socket_name, exiting, realm_id, realm_type, flags, realm_name, address_port, realm_category);
                    return;
                }
            }
        }

        {
            let mut tx_lock = tx.lock().await;
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

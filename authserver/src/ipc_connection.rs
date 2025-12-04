use std::{
    collections::{HashMap, hash_map::Entry},
    io::ErrorKind,
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use interprocess::local_socket::{
    GenericNamespaced, ListenerOptions, ToNsName,
    tokio::SendHalf,
    traits::tokio::{Listener, Stream},
};
use ipc_comms::{
    AuthServerError, AuthServerIpcMessage, GameServerIpcMessage, IpcError, SessionKeyResponse,
    realm_types::{RealmCategory, RealmType},
};
use log::{error, warn};
use tokio::sync::{Mutex, RwLock};

use crate::cache::CacheData;

pub struct RealmData {
    pub realm_type: RealmType,
    pub flags: u8,
    pub realm_name: String,
    pub address_port: SocketAddr,
    pub population: f32,
    pub realm_category: RealmCategory,
}

pub async fn start_ipc_task(
    ipc_socket_name: &str,
    //server_pipes: Arc<Mutex<Vec<Arc<Mutex<Stream>>>>>,
    active_sessions: Arc<RwLock<HashMap<String, (u32, [u8; 40])>>>,
    active_realms: Arc<RwLock<HashMap<u8, RealmData>>>,
    num_player_characters_cache: Arc<RwLock<HashMap<u8, HashMap<u32, CacheData<u8, 60>>>>>,
    server_pipes: Arc<RwLock<HashMap<u8, Arc<Mutex<SendHalf>>>>>,
    exiting: Arc<AtomicBool>,
) {
    let listener_options =
        ListenerOptions::new().name(ipc_socket_name.to_ns_name::<GenericNamespaced>().unwrap());

    let listener = match listener_options.create_tokio() {
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            error!(
                "Could not start server because the socket file is occupied. Please check
                if {} is in use by another process and try again.",
                ipc_socket_name
            );
            return;
        }
        x => x.unwrap(),
    };

    tokio::spawn(async move {
        loop {
            let conn = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to accept an IPC connection: {e}");
                    continue;
                }
            };

            let (mut rx, tx) = conn.split();
            let tx = Arc::new(Mutex::new(tx));

            {
                let tx = tx.clone();
                let active_sessions = active_sessions.clone();
                let active_realms = active_realms.clone();
                let num_player_characters_cache = num_player_characters_cache.clone();
                let server_pipes = server_pipes.clone();
                let exiting = exiting.clone();
                tokio::spawn(async move {
                    let mut saved_realm_id = None;

                    while !exiting.load(Ordering::Relaxed) {
                        let packet = match AuthServerIpcMessage::read(&mut rx).await {
                            Ok(v) => v,
                            Err(IpcError::Io(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                                warn!("Game server (id: {:?}) connection died", saved_realm_id);
                                return;
                            }
                            Err(e) => {
                                error!("Failed to read an IPC message from a game server: {e:?}");
                                continue;
                            }
                        };

                        match packet {
                            AuthServerIpcMessage::RealmInfo {
                                realm_id,
                                realm_type,
                                flags,
                                realm_name,
                                address_port,
                                population,
                                realm_category,
                            } => {
                                // Receiving duplicate realm infos is fine by itself, but realm_id's need to match
                                if saved_realm_id.is_some_and(|v| v != realm_id) {
                                    error!(
                                        "Received a duplicate realm info from a server but the realm ids do not match"
                                    );
                                    let response = GameServerIpcMessage::AuthServerError(
                                        AuthServerError::DuplicateRealmInfo {
                                            previous_id: saved_realm_id.unwrap(),
                                            new_id: realm_id,
                                        },
                                    );

                                    let mut tx_lock = tx.lock().await;

                                    match response.write(&mut *tx_lock).await {
                                        Ok(()) => (),
                                        Err(e) => {
                                            error!(
                                                "Failed to write an IPC message to a game server: {e:?}"
                                            );
                                            continue;
                                        }
                                    };
                                    continue;
                                }

                                if saved_realm_id.is_none() {
                                    saved_realm_id.replace(realm_id);
                                    let mut lock = server_pipes.write().await;
                                    lock.insert(realm_id, tx.clone());
                                }

                                {
                                    let mut lock = active_realms.write().await;
                                    lock.insert(
                                        realm_id,
                                        RealmData {
                                            realm_type,
                                            flags,
                                            realm_name,
                                            address_port,
                                            population,
                                            realm_category,
                                        },
                                    );
                                }
                            }
                            AuthServerIpcMessage::PlayerSessionKeyRequest { account_name } => {
                                let lock = active_sessions.read().await;
                                let session_key = lock.get(&account_name).cloned();
                                drop(lock);

                                let response = GameServerIpcMessage::PlayerSessionKeyResponse {
                                    account_name,
                                    session_key: match session_key {
                                        Some(v) => SessionKeyResponse::Authenticated {
                                            account_id: v.0,
                                            session_key: v.1,
                                        },
                                        None => SessionKeyResponse::Unauthenticated,
                                    },
                                };

                                let mut tx_lock = tx.lock().await;

                                match response.write(&mut *tx_lock).await {
                                    Ok(()) => (),
                                    Err(e) => {
                                        error!(
                                            "Failed to write an IPC message to a game server: {e:?}"
                                        );
                                        continue;
                                    }
                                };
                            }
                            AuthServerIpcMessage::PlayerNumCharactersResponse {
                                account_id,
                                num_characters,
                            } => {
                                let Some(realm_id) = saved_realm_id else {
                                    error!(
                                        "Tried to set number of player characters before sending realm info"
                                    );
                                    continue;
                                };
                                let mut lock = num_player_characters_cache.write().await;
                                let map = lock.entry(realm_id).or_default();
                                match map.entry(account_id) {
                                    Entry::Occupied(mut occupied_entry) => {
                                        occupied_entry.get_mut().update_data(num_characters);
                                    }
                                    Entry::Vacant(vacant_entry) => {
                                        vacant_entry.insert(CacheData::new(num_characters));
                                    }
                                }
                            }
                            AuthServerIpcMessage::GameServerClosed => {
                                if let Some(realm_id) = saved_realm_id {
                                    let mut lock = active_realms.write().await;
                                    lock.remove(&realm_id);
                                }
                                return;
                            }
                        }
                    }

                    {
                        let mut tx_lock = tx.lock().await;
                        let packet = GameServerIpcMessage::AuthServerClosed;
                        match packet.write(&mut *tx_lock).await {
                            Ok(()) => (),
                            Err(e) => {
                                error!("Failed to write an IPC message to a game server: {e:?}");
                            }
                        };
                    }
                });
            }

            /*
            {
                let mut lock = server_pipes.lock().await;
                lock.push(conn);
                drop(lock);
            }*/
        }
    });
}

use std::{collections::HashMap, ffi::CString, sync::Arc, task::Poll};

use concurrent_queue::ConcurrentQueue;
use futures::StreamExt;
use interprocess::local_socket::tokio::SendHalf;
use ipc_comms::GameServerIpcMessage;
use log::{error, warn};
use num_bigint::BigInt;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tokio::{
    io::AsyncWriteExt,
    net::TcpStream,
    sync::{Mutex, RwLock},
};

use crate::{
    cache::CacheData,
    ipc_connection::RealmData,
    packets::{
        self, ClientPacket, ParseError, RealmInfo, ServerPacket,
        opcodes::{CMD_AUTH_LOGON_CHALLENGE, CMD_AUTH_LOGON_PROOF},
    },
    srp,
};

pub async fn handle_connection(
    mut stream: TcpStream,
    server_private_key: BigInt,
    db: DatabaseConnection,
    active_sessions: Arc<RwLock<HashMap<String, [u8; 40]>>>,
    active_realms: Arc<RwLock<HashMap<u8, RealmData>>>,
    num_player_characters_cache: Arc<RwLock<HashMap<u8, HashMap<String, CacheData<u8, 60>>>>>,
    server_pipes: Arc<RwLock<HashMap<u8, Arc<Mutex<SendHalf>>>>>,
) {
    let packet = match ClientPacket::from_reader(&mut stream, Some(CMD_AUTH_LOGON_CHALLENGE)).await
    {
        Ok(v) => v,
        Err(ParseError::UnexpectedOpcode(opcode)) => {
            warn!("Client sent an unexpected opcode: {opcode}");
            return;
        }
        Err(e) => {
            warn!("An error occured while parsing client packet: {:?}", e);
            return;
        }
    };
    #[allow(unused)]
    let ClientPacket::CMD_AUTH_LOGON_CHALLENGE {
        protocol_version,
        game_name,
        version,
        build,
        platform,
        os,
        locale,
        worldregion_bias,
        ip,
        account_name,
    } = packet
    else {
        unreachable!()
    };
    let user = authserver_entity::user::Entity::find()
        .filter(authserver_entity::user::Column::AccountName.eq(account_name.clone()))
        .one(&db)
        .await
        .unwrap();

    let user = if let Some(user) = user { user } else { todo!() };
    let salt: [u8; 32] = user.salt.try_into().unwrap();

    let password_verifier_bigint =
        BigInt::from_bytes_le(num_bigint::Sign::Plus, &user.password_verifier);

    let server_public_key =
        srp::calculate_server_public_key(&password_verifier_bigint, &server_private_key);

    let send_packet = ServerPacket::CMD_AUTH_LOGON_CHALLENGE {
        server_public_key,
        generator: vec![srp::GENERATOR],
        large_safe_prime: srp::LARGE_SAFE_PRIME_LITTLE_ENDIAN.to_vec(),
        salt,
        crc_salt: [0; 16],
        pin: None,
    };

    stream.write_all(&send_packet.to_bytes()).await.unwrap();

    let packet = match ClientPacket::from_reader(&mut stream, Some(CMD_AUTH_LOGON_PROOF)).await {
        Ok(v) => v,
        Err(ParseError::UnexpectedOpcode(opcode)) => {
            warn!("Client sent an unexpected opcode: {opcode}");
            return;
        }
        Err(e) => {
            warn!("An error occured while parsing client packet: {:?}", e);
            return;
        }
    };

    #[rustfmt::skip]
    let (client_public_key, client_proof, _crc_hash, _keys_telemetry, _pin_two_factor_data) = if let ClientPacket::CMD_AUTH_LOGON_PROOF { client_public_key, client_proof, crc_hash, keys_telemetry, pin_two_factor_data } = packet {
        (client_public_key, client_proof, crc_hash, keys_telemetry, pin_two_factor_data)
    } else {
        unreachable!();
    };

    let session_key = srp::calculate_session_key(
        client_public_key,
        server_public_key,
        &password_verifier_bigint,
        &server_private_key,
    );

    let server_calculated_client_proof = srp::calculate_client_proof(
        &account_name,
        salt,
        client_public_key,
        server_public_key,
        session_key,
    );

    if client_proof != server_calculated_client_proof {
        todo!()
    }

    let server_proof = srp::calculate_server_proof(client_public_key, client_proof, session_key);

    let send_packet = ServerPacket::CMD_AUTH_LOGON_PROOF {
        server_proof,
        hardware_survey_id: 0,
    };
    stream.write_all(&send_packet.to_bytes()).await.unwrap();

    {
        let mut lock = active_sessions.write().await;
        lock.insert(account_name.clone(), session_key);
    }

    loop {
        let packet = match ClientPacket::from_reader(&mut stream, None).await {
            Ok(v) => v,
            Err(packets::ParseError::IoError(_e)) => {
                return;
            }
            Err(_) => {
                return;
            }
        };
        match packet {
            ClientPacket::CMD_AUTH_LOGON_CHALLENGE { .. } => (),
            ClientPacket::CMD_AUTH_LOGON_PROOF { .. } => (),
            ClientPacket::CMD_REALM_LIST => {
                let realms_lock = active_realms.read().await;
                let mut realm_infos = Vec::new();

                let mut resolved_num_players = HashMap::new();
                let mut unresolved_num_players = Vec::new();

                {
                    //If num characters is present in cache
                    let num_chars_lock = num_player_characters_cache.read().await;

                    for realm in realms_lock.iter() {
                        if let Some(map) = num_chars_lock.get(&realm.0)
                            && let Some(data) = map.get(&account_name)
                            && data.is_valid()
                        {
                            resolved_num_players.insert(*realm.0, *data.get());
                        } else {
                            unresolved_num_players.push(realm.0);
                        }
                    }
                }

                if unresolved_num_players.len() > 0 {
                    let queue = Arc::new(ConcurrentQueue::bounded(unresolved_num_players.len()));
                    // Otherwise
                    futures::stream::iter(unresolved_num_players)
                        .for_each_concurrent(None, |realm_id| {
                            let account_name = account_name.clone();
                            let num_player_characters_cache = num_player_characters_cache.clone();
                            let server_pipes = server_pipes.clone();
                            let queue = queue.clone();
                            async move {
                                let request = GameServerIpcMessage::PlayerNumCharactersRequest {
                                    account_name: account_name.clone(),
                                };
                                let server_pipes_lock = server_pipes.read().await;
                                let mut lock = server_pipes_lock
                                    .get(&realm_id)
                                    .expect("Realm info was available but its pipe was not")
                                    .lock()
                                    .await;
                                let result = request.write(&mut *lock).await;
                                match result {
                                    Ok(_) => {
                                        let v = NumCharsResolvedFuture::new(
                                            *realm_id,
                                            account_name.clone(),
                                            num_player_characters_cache,
                                        )
                                        .await;
                                        queue.push(Ok(v)).unwrap();
                                    }
                                    Err(e) => {
                                        queue.push(Err(e)).unwrap();
                                    }
                                }
                            }
                        })
                        .await;

                    for result in queue.try_iter() {
                        match result {
                            Ok(v) => {
                                resolved_num_players.insert(v.0, v.1);
                            }
                            Err(e) => {
                                error!("Failed to write an IPC message to a game server: {e:?}");
                                continue;
                            }
                        }
                    }
                }

                for realm in realms_lock
                    .iter()
                    .map(|(realm_id, data)| (realm_id, data, resolved_num_players.get(&realm_id)))
                {
                    let Some(num_chars) = realm.2 else {
                        continue;
                    };
                    realm_infos.push(RealmInfo {
                        realm_type: realm.1.realm_type.clone(),
                        flags: realm.1.flags,
                        realm_name: CString::new(realm.1.realm_name.clone()).unwrap(),
                        address_port: realm.1.address_port,
                        population: realm.1.population,
                        num_chars: *num_chars,
                        realm_category: realm.1.realm_category.clone(),
                        realm_id: *realm.0,
                    })
                }

                let send_packet = ServerPacket::CMD_REALM_LIST {
                    realms: realm_infos,
                };
                stream.write_all(&send_packet.to_bytes()).await.unwrap();
            }
            _ => todo!(),
        }
    }
}

struct NumCharsResolvedFuture {
    realm_id: u8,
    account_name: String,
    num_player_characters_cache: Arc<RwLock<HashMap<u8, HashMap<String, CacheData<u8, 60>>>>>,
}

impl NumCharsResolvedFuture {
    pub fn new(
        realm_id: u8,
        account_name: String,
        num_player_characters_cache: Arc<RwLock<HashMap<u8, HashMap<String, CacheData<u8, 60>>>>>,
    ) -> Self {
        Self {
            realm_id,
            account_name,
            num_player_characters_cache,
        }
    }
}

impl Future for NumCharsResolvedFuture {
    type Output = (u8, u8);

    fn poll(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let Ok(lock) = self.num_player_characters_cache.try_read() else {
            return Poll::Pending;
        };

        if let Some(map) = lock.get(&self.realm_id)
            && let Some(data) = map.get(&self.account_name)
            && data.is_valid()
        {
            Poll::Ready((self.realm_id, *data.get()))
        } else {
            Poll::Pending
        }
    }
}

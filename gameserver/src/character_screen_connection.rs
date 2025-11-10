use std::{collections::HashMap, sync::Arc};

use interprocess::local_socket::tokio::SendHalf;
use ipc_comms::{AuthServerIpcMessage, SessionKeyResponse};
use lazy_static::lazy_static;
use packets::{account_result::AccountResult, client::ParseError};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use sha1::{Digest, Sha1};
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};

lazy_static! {
    static ref SECURE_RNG: Mutex<StdRng> = Mutex::new(StdRng::from_os_rng());
}

pub struct CharacterScreenConnection {
    pub account_id: u32,
    pub stream: TcpStream,
}

impl CharacterScreenConnection {
    pub async fn authenticate(
        mut stream: TcpStream,
        player_session_keys: Arc<Mutex<HashMap<String, SessionKeyResponse>>>,
        server_pipe: Arc<Mutex<Option<SendHalf>>>,
    ) -> Result<Self, ParseError> {
        let server_seed = SECURE_RNG.lock().await.next_u32();
        let send_packet = packets::server::SMSG_AUTH_CHALLENGE { server_seed };
        stream
            .write_all(&send_packet.to_bytes(None))
            .await
            .map_err(ParseError::Io)?;

        let packet =
            packets::client::read_specific_packet::<_, packets::client::CMSG_AUTH_SESSION>(
                &mut stream,
                None,
            )
            .await?;

        let account_name = packet.username.to_str().unwrap().to_string(); //TODO: figure out how to handle this properly

        let (account_id, session_key) = {
            let request = AuthServerIpcMessage::PlayerSessionKeyRequest {
                account_name: account_name.clone(),
            };
            let mut lock = server_pipe.lock().await;
            let Some(ref mut pipe) = *lock else {
                todo!() //TODO: kick a player since authserver connection is gone
            };
            request.write(pipe).await.unwrap(); //TODO: gracefully kick player in case this fails as this is an internal error

            // TODO: Figure out a better solution instead of spin locking
            // Futures is a solution but passing the waker around is way out of scope for now

            let v;
            loop {
                let mut lock = player_session_keys.lock().await;
                if let Some(map) = lock.remove(&account_name) {
                    match map {
                        SessionKeyResponse::Authenticated {
                            account_id,
                            session_key,
                        } => {
                            v = (account_id, session_key);
                        }
                        SessionKeyResponse::Unauthenticated => {
                            //TODO: kick player for not being authenticated
                            todo!()
                        }
                    }
                    break;
                }
                drop(lock);
            }

            v
        };

        let server_proof: [u8; 20] = calculate_world_server_proof(
            account_name,
            packet.client_seed,
            server_seed,
            session_key,
        );
        if packet.client_proof != server_proof {
            todo!();
        }

        let send_packet = packets::server::SMSG_AUTH_RESPONSE {
            result: AccountResult::AUTH_OK {
                billing_time: 50000,
                billing_flags: 0,
                billing_rested: 0,
            },
        };
        stream
            .write_all(&send_packet.to_bytes(None))
            .await
            .map_err(ParseError::Io)?;

        Ok(Self { account_id, stream })
    }
}

fn calculate_world_server_proof(
    username: String,
    client_seed: u32,
    server_seed: u32,
    session_key: [u8; 40],
) -> [u8; 20] {
    Sha1::new()
        .chain_update(username.as_bytes())
        .chain_update(0u32.to_le_bytes())
        .chain_update(client_seed.to_le_bytes())
        .chain_update(server_seed.to_le_bytes())
        .chain_update(session_key)
        .finalize()
        .into()
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::character_screen_connection::calculate_world_server_proof;

    #[test]
    fn calc_world_server_proof() {
        let f = include_str!("../tests/calculate_world_server_proof.txt");
        for line in f.split("\n") {
            if line.is_empty() {
                continue;
            }
            let (username, session_key, server_seed, client_seed, expected) =
                line.split(" ").collect_tuple().unwrap();
            let mut session_key = hex::decode(session_key).unwrap();
            session_key.reverse();

            let expected = hex::decode(expected).unwrap();
            assert_eq!(
                calculate_world_server_proof(
                    username.to_owned(),
                    u32::from_le_bytes(hex::decode(client_seed).unwrap().try_into().unwrap()),
                    u32::from_le_bytes(hex::decode(server_seed).unwrap().try_into().unwrap()),
                    session_key.try_into().unwrap()
                ),
                TryInto::<[u8; 20]>::try_into(expected).unwrap()
            );
        }
    }
}

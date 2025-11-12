use std::{collections::HashMap, io::ErrorKind, sync::Arc};

use interprocess::local_socket::tokio::SendHalf;
use ipc_comms::{AuthServerIpcMessage, SessionKeyResponse};
use lazy_static::lazy_static;
use log::{error, info, warn};
use packets::{
    account_result::AccountResult,
    client::{ClientPacket, ParseError},
    guid::{self, Guid},
};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter};
use sha1::{Digest, Sha1};
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};

use crate::{character::NewCharacter, game_data::GameDataAccessor};

lazy_static! {
    static ref SECURE_RNG: Mutex<StdRng> = Mutex::new(StdRng::from_os_rng());
}

pub struct CharacterScreenConnection {
    pub account_id: u32,
    pub stream: TcpStream,
    session_key: [u8; 40],

    db: DatabaseConnection,
    game_data_accessor: GameDataAccessor,
}

impl CharacterScreenConnection {
    pub async fn authenticate(
        mut stream: TcpStream,
        player_session_keys: Arc<Mutex<HashMap<String, SessionKeyResponse>>>,
        server_pipe: Arc<Mutex<Option<SendHalf>>>,
        db: DatabaseConnection,
        game_data_accessor: GameDataAccessor,
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
            {
                let mut lock = server_pipe.lock().await;
                let Some(ref mut pipe) = *lock else {
                    todo!() //TODO: kick the player since authserver connection is gone
                };
                request.write(pipe).await.unwrap(); //TODO: gracefully kick player in case this fails as this is an internal error
            }

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
                            //TODO: kick the player for not being authenticated
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
            .write_all(&send_packet.to_bytes(Some(session_key)))
            .await
            .map_err(ParseError::Io)?;

        Ok(Self {
            account_id,
            stream,
            session_key,

            db,
            game_data_accessor,
        })
    }

    // This function returns when the client has selected the character to allow for state transition
    pub async fn connection_loop(&mut self) -> CharacterScreenResult {
        loop {
            let packet =
                match packets::client::read_packet(&mut self.stream, self.session_key).await {
                    Ok(v) => v,
                    Err(ParseError::Io(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                        return CharacterScreenResult::ClientDisconnect;
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse a packet from a client (account_id: {}). Error: {:?}",
                            self.account_id, e
                        );
                        continue;
                    }
                };

            match packet {
                ClientPacket::CMSG_CHAR_ENUM(_) => {
                    let characters = match gameserver_entity::character::Entity::find()
                        .filter(gameserver_entity::character::Column::AccountId.eq(self.account_id))
                        .all(&self.db)
                        .await
                    {
                        Ok(v) => v,
                        Err(e) => {
                            error!(
                                "Failed to get client's characters from the database. Error: {}",
                                e
                            );
                            continue;
                        }
                    };

                    let response = packets::server::SMSG_CHAR_ENUM {
                        characters: characters
                            .into_iter()
                            .map(|v| crate::character::Character::from_db(v).to_packet())
                            .collect(),
                    };

                    if let Err(e) = self
                        .stream
                        .write_all(&response.to_bytes(Some(self.session_key)))
                        .await
                    {
                        warn!(
                            "Failed to send SMSG_CHAR_ENUM to client (account_id: {}). Error: {:?}",
                            self.account_id, e
                        )
                    };
                }
                ClientPacket::CMSG_CHAR_CREATE(packet) => {
                    let race = match self.game_data_accessor.validate_race(packet.race).await {
                        Ok(Some(v)) => v,
                        Ok(None) => {
                            let response = packets::server::SMSG_CHAR_CREATE {
                                result: AccountResult::CHAR_CREATE_ERROR,
                            };

                            warn!(
                                "Client (account_id: {}) tried to create a character with an invalid race ({})",
                                self.account_id, packet.race
                            );

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                error!(
                                    "Failed to send SMSG_CHAR_CREATE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                        Err(e) => {
                            error!(
                                "Failed to check if provided race was valid due to a DB error. Error: {}",
                                e
                            );
                            let response = packets::server::SMSG_CHAR_CREATE {
                                result: AccountResult::CHAR_CREATE_FAILED,
                            };

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                error!(
                                    "Failed to send SMSG_CHAR_CREATE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                    };
                    let class = match self.game_data_accessor.validate_class(packet.class).await {
                        Ok(Some(v)) => v,
                        Ok(None) => {
                            let response = packets::server::SMSG_CHAR_CREATE {
                                result: AccountResult::CHAR_CREATE_ERROR,
                            };

                            warn!(
                                "Client (account_id: {}) tried to create a character with an invalid class ({})",
                                self.account_id, packet.class
                            );

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                error!(
                                    "Failed to send SMSG_CHAR_CREATE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                        Err(e) => {
                            error!(
                                "Failed to check if provided class was valid due to a DB error. Error: {}",
                                e
                            );
                            let response = packets::server::SMSG_CHAR_CREATE {
                                result: AccountResult::CHAR_CREATE_FAILED,
                            };

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                error!(
                                    "Failed to send SMSG_CHAR_CREATE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                    };
                    let gender = match self.game_data_accessor.validate_gender(packet.gender).await
                    {
                        Ok(Some(v)) => v,
                        Ok(None) => {
                            let response = packets::server::SMSG_CHAR_CREATE {
                                result: AccountResult::CHAR_CREATE_ERROR,
                            };

                            warn!(
                                "Client (account_id: {}) tried to create a character with an invalid gender ({})",
                                self.account_id, packet.gender
                            );

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                error!(
                                    "Failed to send SMSG_CHAR_CREATE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                        Err(e) => {
                            error!(
                                "Failed to check if provided gender was valid due to a DB error. Error: {}",
                                e
                            );
                            let response = packets::server::SMSG_CHAR_CREATE {
                                result: AccountResult::CHAR_CREATE_FAILED,
                            };

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                error!(
                                    "Failed to send SMSG_CHAR_CREATE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                    };
                    //TODO: all the validate_ function calls must be joined and done in parallel
                    //TODO: validate name
                    let name = packet.character_name.to_string_lossy().to_string();
                    //TODO: validate skin, face, hairstyle, etc.

                    let start_char_info = match self
                        .game_data_accessor
                        .get_character_start_data(race, class)
                        .await
                    {
                        Ok(Some(v)) => v,
                        Ok(None) => {
                            let response = packets::server::SMSG_CHAR_CREATE {
                                result: AccountResult::CHAR_CREATE_ERROR,
                            };

                            warn!(
                                "Client (account_id: {}) tried to create a character with an invalid race+class pair (race {} + class {})",
                                self.account_id,
                                race.get(),
                                class.get()
                            );

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                warn!(
                                    "Failed to send SMSG_CHAR_CREATE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                        Err(e) => {
                            error!(
                                "Failed to check if provided gender was valid due to a DB error. Error: {}",
                                e
                            );
                            let response = packets::server::SMSG_CHAR_CREATE {
                                result: AccountResult::CHAR_CREATE_FAILED,
                            };

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                error!(
                                    "Failed to send SMSG_CHAR_CREATE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                    };

                    let character = NewCharacter {
                        name: name.clone(),
                        race,
                        class,
                        gender,
                        skin: packet.skin,
                        face: packet.face,
                        hairstyle: packet.hairstyle,
                        haircolor: packet.haircolor,
                        facialhair: packet.facialhair,
                        level: start_char_info.level,
                        area: start_char_info.area_id,
                        map: start_char_info.map_id,
                        position_x: start_char_info.position.0,
                        position_y: start_char_info.position.1,
                        position_z: start_char_info.position.2,
                        orientation: start_char_info.orientation,
                        guild_id: 0,
                        flags: 0,
                        first_login: true,
                        equipment: start_char_info.start_equipment,
                    };

                    if let Err(e) = character.insert(&self.db, self.account_id).await {
                        error!(
                            "Failed to insert a player's character into database. Error: {}",
                            e
                        );
                        let response = packets::server::SMSG_CHAR_CREATE {
                            result: AccountResult::CHAR_CREATE_FAILED,
                        };

                        if let Err(e) = self
                            .stream
                            .write_all(&response.to_bytes(Some(self.session_key)))
                            .await
                        {
                            warn!(
                                "Failed to send SMSG_CHAR_CREATE to client (account_id: {}). Error: {:?}",
                                self.account_id, e
                            )
                        };
                        continue;
                    }

                    info!(
                        "Client (account_id: {}) has craeted a new character (character_name: {})",
                        self.account_id, name
                    );

                    let response = packets::server::SMSG_CHAR_CREATE {
                        result: AccountResult::CHAR_CREATE_SUCCESS,
                    };

                    if let Err(e) = self
                        .stream
                        .write_all(&response.to_bytes(Some(self.session_key)))
                        .await
                    {
                        warn!(
                            "Failed to send SMSG_CHAR_CREATE to client (account_id: {}). Error: {:?}",
                            self.account_id, e
                        )
                    };
                }
                ClientPacket::CMSG_CHAR_DELETE(packet) => {
                    let Some(guid) = Guid::<guid::Player>::try_from_u64(packet.character_guid)
                    else {
                        let response = packets::server::SMSG_CHAR_DELETE {
                            result: AccountResult::CHAR_DELETE_FAILED,
                        };

                        warn!(
                            "Client (account_id: {}) tried to delete a character with an invalid guid ({})",
                            self.account_id, packet.character_guid
                        );

                        if let Err(e) = self
                            .stream
                            .write_all(&response.to_bytes(Some(self.session_key)))
                            .await
                        {
                            error!(
                                "Failed to send SMSG_CHAR_DELETE to client (account_id: {}). Error: {:?}",
                                self.account_id, e
                            )
                        };
                        continue;
                    };
                    let model = match gameserver_entity::character::Entity::find_by_id(
                        guid.get_u32().get() as i32,
                    )
                    .filter(gameserver_entity::character::Column::AccountId.eq(self.account_id))
                    .one(&self.db)
                    .await
                    {
                        Ok(Some(v)) => v,
                        Ok(None) => {
                            let response = packets::server::SMSG_CHAR_DELETE {
                                result: AccountResult::CHAR_DELETE_FAILED,
                            };

                            warn!(
                                "Client (account_id: {}) tried to delete a character that doesn't exist or not from their account (guid: {})",
                                self.account_id,
                                guid.get_u32()
                            );

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                warn!(
                                    "Failed to send SMSG_CHAR_DELETE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                        Err(e) => {
                            error!(
                                "Failed to get client's character due to a DB error (account_id: {}, character_id: {}). Error: {}",
                                self.account_id,
                                guid.get_u32(),
                                e
                            );
                            let response = packets::server::SMSG_CHAR_DELETE {
                                result: AccountResult::CHAR_DELETE_FAILED,
                            };

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                error!(
                                    "Failed to send SMSG_CHAR_DELETE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                    };

                    match model.delete(&self.db).await {
                        Ok(_) => {
                            info!(
                                "Client (account_id: {}) has deleted a character (character_id: {})",
                                self.account_id,
                                guid.get_u32()
                            );

                            let response = packets::server::SMSG_CHAR_DELETE {
                                result: AccountResult::CHAR_DELETE_SUCCESS,
                            };

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                warn!(
                                    "Failed to send SMSG_CHAR_DELETE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                        Err(e) => {
                            error!(
                                "Failed to delete client's character due to a DB error (account_id: {}, character_id: {}). Error: {}",
                                self.account_id,
                                guid.get_u32(),
                                e
                            );

                            let response = packets::server::SMSG_CHAR_DELETE {
                                result: AccountResult::CHAR_DELETE_FAILED,
                            };

                            if let Err(e) = self
                                .stream
                                .write_all(&response.to_bytes(Some(self.session_key)))
                                .await
                            {
                                warn!(
                                    "Failed to send SMSG_CHAR_DELETE to client (account_id: {}). Error: {:?}",
                                    self.account_id, e
                                )
                            };
                            continue;
                        }
                    };
                }
                ClientPacket::CMSG_PLAYER_LOGIN(packet) => {
                    let Some(guid) = Guid::<guid::Player>::try_from_u64(packet.character_guid)
                    else {
                        let response = packets::server::SMSG_CHAR_LOGIN_FAILED {
                            result: AccountResult::CHAR_LOGIN_NO_CHARACTER,
                        };

                        warn!(
                            "Client (account_id: {}) tried to log into a character with an invalid guid ({})",
                            self.account_id, packet.character_guid
                        );

                        if let Err(e) = self
                            .stream
                            .write_all(&response.to_bytes(Some(self.session_key)))
                            .await
                        {
                            error!(
                                "Failed to send SMSG_CHAR_LOGIN_FAILED to client (account_id: {}). Error: {:?}",
                                self.account_id, e
                            )
                        };
                        continue;
                    };

                    todo!()
                }
                ClientPacket::CMSG_PING(packet) => {
                    let response = packets::server::SMSG_PONG {
                        sequence_id: packet.sequence_id,
                    };

                    if let Err(e) = self
                        .stream
                        .write_all(&response.to_bytes(Some(self.session_key)))
                        .await
                    {
                        warn!(
                            "Failed to send SMSG_PONG to client (account_id: {}). Error: {:?}",
                            self.account_id, e
                        )
                    };
                }
                _ => {
                    warn!(
                        "Client (account_id: {}) tried to send a packet in a wrong state (current state: character screen). Packet: {:?}",
                        self.account_id, packet
                    );
                }
            }
        }
    }
}

pub enum CharacterScreenResult {
    WorldTransition,
    ClientDisconnect,
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

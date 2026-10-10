use std::{ffi::CString, io::ErrorKind, sync::Arc};

use chrono::Local;
use common::guid::{self, Guid, LivingGuid, SelectableGuid};
use concurrent_queue::ConcurrentQueue;
use gameobjects::{
    player::TutorialFlags,
    unit::{SheathState, StandStateType},
};
use log::{error, warn};
use packets::{
    chat_message::ChatType,
    client::{ClientPacket, ParseError},
    movement_info::MovementInfo,
    server::ServerPacket,
};
use sqlx::{Pool, Sqlite};
use tokio::{
    io::AsyncWriteExt,
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::Mutex,
};

use crate::game_data::{GameDataAccessor, ValidLanguage};

pub async fn packet_handler(
    mut rx: OwnedReadHalf,
    tx: Arc<Mutex<OwnedWriteHalf>>,
    session_key: [u8; 40],
    mut decrypt_data: (usize, u8),
    encrypt_data: Arc<Mutex<(usize, u8)>>,
    character_id: Guid<guid::Player>,
    player_update_queue: Arc<ConcurrentQueue<PlayerUpdate>>,
    game_data_accessor: GameDataAccessor,
    db: Pool<Sqlite>,
) {
    loop {
        let packet =
            match packets::client::read_packet(&mut rx, session_key, &mut decrypt_data).await {
                Ok(v) => v,
                Err(ParseError::Io(e)) if e.kind() == ErrorKind::UnexpectedEof => {
                    let _ = player_update_queue.push(PlayerUpdate {
                        character_id: character_id,
                        data: PlayerUpdateData::ForceKick,
                    });
                    return;
                }
                Err(e) => {
                    warn!(
                        "Failed to parse a packet from a client (character_id: {}). Error: {:?}",
                        character_id.get_u32(),
                        e
                    );
                    continue;
                }
            };

        match packet {
            // Movement
            ClientPacket::MSG_MOVE_START_FORWARD(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StartForward,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_BACKWARD(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StartBackward,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_STOP(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::Stop,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_STRAFE_LEFT(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StartStrafeLeft,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_STRAFE_RIGHT(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StartStrafeRight,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_STOP_STRAFE(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StopStrafe,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_JUMP(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::Jump,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_TURN_LEFT(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StartTurnLeft,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_TURN_RIGHT(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StartTurnRight,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_STOP_TURN(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StopTurn,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_PITCH_UP(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StartPitchUp,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_PITCH_DOWN(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StartPitchDown,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_STOP_PITCH(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StopPitch,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_SET_RUN_MODE(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::SetRunMode,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_SET_WALK_MODE(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::SetWalkMode,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_FALL_LAND(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::FallLand,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_START_SWIM(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StartSwim,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_STOP_SWIM(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::StopSwim,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_SET_FACING(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::SetFacing,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_SET_PITCH(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::SetPitch,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::MSG_MOVE_HEARTBEAT(packet) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::Movement {
                        opcode: MovementOpcode::Heartbeat,
                        movement_info: packet.movement_info,
                    },
                }) {
                    return;
                };
            }
            // Movement processing end
            ClientPacket::CMSG_SETSHEATHED(packet) => {
                let state = match SheathState::try_from_bits(packet.sheath_state as u8) {
                    Ok(v) => v,
                    Err(value) => {
                        warn!(
                            "Tried to convert an incorrect value ({}) to SheathState. (character_id: {})",
                            value,
                            character_id.get_u32()
                        );
                        if let Err(_) = player_update_queue.push(PlayerUpdate {
                            character_id,
                            data: PlayerUpdateData::ResendSheathState,
                        }) {
                            return;
                        };
                        continue;
                    }
                };

                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::SetSheathState { state: state },
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_STANDSTATECHANGE(packet) => {
                let state = match StandStateType::try_from_bits(packet.anim_state as u8) {
                    Ok(v) => v,
                    Err(value) => {
                        warn!(
                            "Tried to convert an incorrect value ({}) to StandStateType. (character_id: {})",
                            value,
                            character_id.get_u32()
                        );
                        if let Err(_) = player_update_queue.push(PlayerUpdate {
                            character_id,
                            data: PlayerUpdateData::ResendAnimationState,
                        }) {
                            return;
                        };
                        continue;
                    }
                };

                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::SetAnimationState { state: state },
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_AUTOEQUIP_ITEM(packet) => {
                if packet.src_bag != 0xff {
                    todo!()
                }

                let src_slot = parse_slot(packet.src_slot);

                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::AutoEquipItem { src: src_slot },
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_AUTOSTORE_BAG_ITEM(packet) => {
                if packet.src_bag != 0xff || packet.dst_bag != 0xff {
                    todo!()
                }

                let src_slot = parse_slot(packet.src_slot);

                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::StoreItemInBackpack { src: src_slot },
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_SWAP_INV_ITEM(packet) => {
                let src_slot = parse_slot(packet.src_slot);
                let dst_slot = parse_slot(packet.dst_slot);

                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::SwapInventoryItem {
                        src: src_slot,
                        dst: dst_slot,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_NAME_QUERY(packet) => {
                let Some(guid) = Guid::<guid::Player>::try_from_u64(packet.guid) else {
                    warn!(
                        "Client (character_id: {}) sent CMSG_NAME_QUERY with a non-player guid: {}",
                        character_id.get(),
                        packet.guid
                    );
                    continue;
                };

                let response = match CharacterNameQueryData::load_from_db(&db, guid).await {
                    Ok(Some(data)) => packets::server::SMSG_NAME_QUERY_RESPONSE {
                        guid,
                        name: Some(CString::new(data.name).unwrap()),
                        realm_name: None,
                        race: data.race as u32,
                        gender: data.gender as u32,
                        class: data.class as u32,
                    },
                    Ok(None) => packets::server::SMSG_NAME_QUERY_RESPONSE {
                        guid,
                        name: None,
                        realm_name: None,
                        race: 0,
                        gender: 0,
                        class: 0,
                    },
                    Err(e) => {
                        error!(
                            "Failed to query character due to a DB error (target: {}, requesting character_id: {}). Error: {}",
                            guid.get(),
                            character_id.get(),
                            e
                        );
                        continue;
                    }
                };

                let mut lock = tx.lock().await;

                if let Err(e) = lock
                    .write_all(
                        &response.to_bytes(Some(session_key), &mut *encrypt_data.lock().await),
                    )
                    .await
                {
                    warn!(
                        "Failed to send SMSG_NAME_QUERY_RESPONSE to client (character_id: {}). Error: {:?}",
                        character_id.get(),
                        e
                    )
                };
            }
            ClientPacket::CMSG_ITEM_QUERY_SINGLE(packet) => {
                let item_id = packet.item_id;

                let item_prototype = match game_data_accessor.get_item_prototype(item_id).await {
                    Ok(Some(v)) => v,
                    Ok(None) => {
                        continue;
                    }
                    Err(e) => {
                        error!(
                            "Failed to query item prototype due to a DB error (item_id: {}, requesting character_id: {}). Error: {}",
                            item_id,
                            character_id.get(),
                            e
                        );
                        continue;
                    }
                };

                let response = packets::server::SMSG_ITEM_QUERY_SINGLE_RESPONSE {
                    item_id,
                    class: item_prototype.class as u32,
                    sub_class: item_prototype.sub_class,
                    name_1: CString::new(item_prototype.name).unwrap(),
                    name_2: CString::new("").unwrap(),
                    name_3: CString::new("").unwrap(),
                    name_4: CString::new("").unwrap(),
                    display_info_id: item_prototype.display_info_id,
                    quality: item_prototype.quality,
                    flags: item_prototype.flags,
                    buy_price: item_prototype.buy_price,
                    sell_price: item_prototype.sell_price,
                    inventory_type: item_prototype.inventory_type as u32,
                    allowable_class: item_prototype.allowable_class,
                    allowable_race: item_prototype.allowable_race,
                    item_level: item_prototype.item_level,
                    required_level: item_prototype.required_level,
                    required_skill: item_prototype.required_skill,
                    required_skill_rank: item_prototype.required_skill_rank,
                    required_spell: item_prototype.required_spell,
                    required_honor_rank: item_prototype.required_honor_rank,
                    required_city_rank: item_prototype.required_city_rank,
                    required_reputation_faction: item_prototype.required_reputation_faction,
                    required_reputation_rank: item_prototype.required_reputation_rank,
                    max_count: item_prototype.max_count,
                    stackable: item_prototype.stackable,
                    container_slots: item_prototype.container_slots,
                    item_stats: item_prototype.item_stats,
                    damage: item_prototype.damage,
                    armor: item_prototype.armor,
                    holy_resistance: item_prototype.holy_resistance,
                    fire_resistance: item_prototype.fire_resistance,
                    nature_resistance: item_prototype.nature_resistance,
                    frost_resistance: item_prototype.frost_resistance,
                    shadow_resistance: item_prototype.shadow_resistance,
                    arcane_resistance: item_prototype.arcane_resistance,
                    delay: item_prototype.delay,
                    ammo_type: item_prototype.ammo_type,
                    ranged_mod_range: item_prototype.ranged_mod_range,
                    spells: item_prototype.spells,
                    bonding: item_prototype.bonding,
                    description: CString::new(item_prototype.description).unwrap(),
                    page_text: item_prototype.page_text,
                    language_id: item_prototype.language_id,
                    page_material: item_prototype.page_material,
                    start_quest: item_prototype.start_quest,
                    lock_id: item_prototype.lock_id,
                    material: item_prototype.material,
                    sheath: item_prototype.sheath,
                    random_property: item_prototype.random_property,
                    block: item_prototype.block,
                    item_set: item_prototype.item_set,
                    max_durability: item_prototype.max_durability,
                    area: item_prototype.area,
                    map: item_prototype.map,
                    bag_family: item_prototype.bag_family,
                };

                let mut lock = tx.lock().await;

                if let Err(e) = lock
                    .write_all(
                        &response.to_bytes(Some(session_key), &mut *encrypt_data.lock().await),
                    )
                    .await
                {
                    warn!(
                        "Failed to send SMSG_ITEM_QUERY_SINGLE_RESPONSE to client (character_id: {}). Error: {:?}",
                        character_id.get(),
                        e
                    )
                };
            }
            ClientPacket::CMSG_QUERY_TIME(_) => {
                let response = packets::server::SMSG_QUERY_TIME_RESPONSE { time: Local::now() };

                let mut lock = tx.lock().await;

                if let Err(e) = lock
                    .write_all(
                        &response.to_bytes(Some(session_key), &mut *encrypt_data.lock().await),
                    )
                    .await
                {
                    warn!(
                        "Failed to send SMSG_PONG to client (character_id: {}). Error: {:?}",
                        character_id.get(),
                        e
                    )
                };
            }
            ClientPacket::CMSG_PING(packet) => {
                let response = packets::server::SMSG_PONG {
                    sequence_id: packet.sequence_id,
                };

                let mut lock = tx.lock().await;

                if let Err(e) = lock
                    .write_all(
                        &response.to_bytes(Some(session_key), &mut *encrypt_data.lock().await),
                    )
                    .await
                {
                    warn!(
                        "Failed to send SMSG_PONG to client (character_id: {}). Error: {:?}",
                        character_id.get(),
                        e
                    )
                };
            }
            ClientPacket::CMSG_CREATURE_QUERY(packet) => {
                let creature_id = packet.creature_id;

                let creature_prototype = match game_data_accessor
                    .get_creature_prototype(creature_id)
                    .await
                {
                    Ok(Some(v)) => v,
                    Ok(None) => {
                        continue;
                    }
                    Err(e) => {
                        error!(
                            "Failed to query creature prototype due to a DB error (creature_id: {}, requesting character_id: {}). Error: {}",
                            creature_id,
                            character_id.get(),
                            e
                        );
                        continue;
                    }
                };

                let response = packets::server::SMSG_CREATURE_QUERY_RESPONSE {
                    creature_id,
                    name_1: CString::new(creature_prototype.name).unwrap(),
                    name_2: CString::new("").unwrap(),
                    name_3: CString::new("").unwrap(),
                    name_4: CString::new("").unwrap(),
                    sub_name: CString::new(creature_prototype.sub_name.unwrap_or(String::new()))
                        .unwrap(),
                    flags: creature_prototype.flags,
                    creature_type: creature_prototype.r#type,
                    family: creature_prototype.family,
                    rank: creature_prototype.rank,
                    unkn: 0,
                    pet_spell_data_id: 0,
                    display_id: creature_prototype.display_id,
                    civilian: creature_prototype.civilian,
                };

                let mut lock = tx.lock().await;

                if let Err(e) = lock
                    .write_all(
                        &response.to_bytes(Some(session_key), &mut *encrypt_data.lock().await),
                    )
                    .await
                {
                    warn!(
                        "Failed to send SMSG_CREATURE_QUERY_RESPONSE to client (character_id: {}). Error: {:?}",
                        character_id.get(),
                        e
                    )
                };
            }
            ClientPacket::CMSG_LOGOUT_REQUEST(_) => {
                // TODO: logout checks

                let response = packets::server::SMSG_LOGOUT_RESPONSE {
                    failed: 0,
                    instant: 1,
                };

                let mut lock = tx.lock().await;

                if let Err(e) = lock
                    .write_all(
                        &response.to_bytes(Some(session_key), &mut *encrypt_data.lock().await),
                    )
                    .await
                {
                    warn!(
                        "Failed to send SMSG_LOGOUT_RESPONSE to client (character_id: {}). Error: {:?}",
                        character_id.get(),
                        e
                    )
                };

                let response = packets::server::SMSG_LOGOUT_COMPLETE {};

                if let Err(e) = lock
                    .write_all(
                        &response.to_bytes(Some(session_key), &mut *encrypt_data.lock().await),
                    )
                    .await
                {
                    warn!(
                        "Failed to send SMSG_LOGOUT_COMPLETE to client (character_id: {}). Error: {:?}",
                        character_id.get(),
                        e
                    )
                };

                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::TransitionToCharacterScreen { rx, decrypt_data },
                }) {
                    return;
                };
                return;
            }
            ClientPacket::CMSG_SET_SELECTION(packet) => {
                //TODO: check that guid actually resolves to something
                let guid = SelectableGuid::try_from_u64(packet.guid);
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::SetSelection { guid },
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_TUTORIAL_FLAG(packet) => {
                if packet.i_flag >= TutorialFlags::COUNT {
                    warn!(
                        "Client (character_id: {}) sent an invalid tutorial flag: {}",
                        character_id.get(),
                        packet.i_flag
                    );
                    continue;
                }

                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::SetTutorialFlag {
                        flag: packet.i_flag,
                    },
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_TUTORIAL_CLEAR(_) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::ClearTutorials,
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_TUTORIAL_RESET(_) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::ResetTutorials,
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_ATTACKSWING(packet) => {
                let Some(victim) = LivingGuid::try_from_u64(packet.guid) else {
                    warn!(
                        "Client (character_id: {}) sent CMSG_ATTACKSWING with an invalid guid: {}",
                        character_id.get(),
                        packet.guid
                    );

                    let response = packets::server::SMSG_ATTACKSWING_CANT_ATTACK {};

                    if let Err(e) = tx
                        .lock()
                        .await
                        .write_all(
                            &response.to_bytes(Some(session_key), &mut *encrypt_data.lock().await),
                        )
                        .await
                    {
                        warn!(
                            "Failed to send SMSG_ATTACKSWING_CANT_ATTACK to client (character_id: {}). Error: {:?}",
                            character_id.get(),
                            e
                        )
                    };
                    continue;
                };

                //TODO: check that guid actually resolves to something

                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::StartCombat { victim },
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_ATTACKSTOP(_) => {
                if let Err(_) = player_update_queue.push(PlayerUpdate {
                    character_id,
                    data: PlayerUpdateData::StopCombat,
                }) {
                    return;
                };
            }
            ClientPacket::CMSG_MESSAGECHAT(packet) => {
                if !packet.chat.is_addon_message() {
                    match packet.chat.ty {
                        ChatType::AFK => {
                            if let Err(_) = player_update_queue.push(PlayerUpdate {
                                character_id,
                                data: PlayerUpdateData::SetAFK {
                                    message: packet.chat.message,
                                },
                            }) {
                                return;
                            };
                            continue;
                        }
                        ChatType::DND => {
                            if let Err(_) = player_update_queue.push(PlayerUpdate {
                                character_id,
                                data: PlayerUpdateData::SetDND {
                                    message: packet.chat.message,
                                },
                            }) {
                                return;
                            };
                            continue;
                        }
                        _ => (),
                    };

                    let language = match game_data_accessor
                        .validate_language(packet.chat.lang)
                        .await
                    {
                        Ok(Some(language)) => language,
                        Ok(None) => {
                            warn!(
                                "Client (character_id: {}) sent CMSG_MESSAGECHAT with an invalid language: {}",
                                character_id.get(),
                                packet.chat.lang
                            );
                            continue;
                        }
                        Err(e) => {
                            error!(
                                "Failed to query language due to a DB error (language: {}, requesting character_id: {}). Error: {}",
                                packet.chat.lang,
                                character_id.get(),
                                e
                            );
                            continue;
                        }
                    };

                    let text = packet.chat.message;
                    let message = match packet.chat.ty {
                        ChatType::Say => ChatMessage::Say { language, text },
                        ChatType::Party => ChatMessage::Party { language, text },
                        ChatType::Raid => ChatMessage::Raid { language, text },
                        ChatType::Guild => ChatMessage::Guild { language, text },
                        ChatType::Officer => ChatMessage::Officer { language, text },
                        ChatType::Yell => ChatMessage::Yell { language, text },
                        ChatType::Whisper { target_name } => ChatMessage::Whisper {
                            language,
                            target_name,
                            text,
                        },
                        ChatType::Emote => ChatMessage::Emote { language, text },
                        ChatType::Channel { channel_name } => ChatMessage::Channel {
                            language,
                            channel_name,
                            text,
                        },
                        ChatType::RaidLeader => ChatMessage::RaidLeader { language, text },
                        ChatType::RaidWarning => ChatMessage::RaidWarning { language, text },
                        ChatType::BattleGround => ChatMessage::BattleGround { language, text },
                        ChatType::BattleGroundLeader => {
                            ChatMessage::BattleGroundLeader { language, text }
                        }
                        ty => {
                            warn!("Unprocessed chat type: {:?}", ty);
                            continue;
                        }
                    };

                    if let Err(_) = player_update_queue.push(PlayerUpdate {
                        character_id,
                        data: PlayerUpdateData::ChatMessage(message),
                    }) {
                        return;
                    };
                }
            }
            _ => {
                warn!(
                    "Client (character_id: {}) tried to send a packet in a wrong state (current state: game world). Packet: {:?}",
                    character_id.get(),
                    packet
                );
            }
        }
    }
}

pub struct PlayerUpdate {
    pub character_id: Guid<guid::Player>,
    pub data: PlayerUpdateData,
}

pub enum PlayerUpdateData {
    Movement {
        opcode: MovementOpcode,
        movement_info: MovementInfo,
    },
    SwapInventoryItem {
        src: Slot,
        dst: Slot,
    },
    AutoEquipItem {
        src: Slot,
    },
    // Moves the item into the first free backpack slot
    StoreItemInBackpack {
        src: Slot,
    },
    // In case CMSG_SETSHEATHED failed to validate StandStateType
    ResendSheathState,
    SetSheathState {
        state: SheathState,
    },
    // In case CMSG_STANDSTATECHANGE failed to validate StandStateType
    ResendAnimationState,
    SetAnimationState {
        state: StandStateType,
    },
    // Kicks the client by abruptly dropping their connection, usually due to an error in reading client's packets
    ForceKick,
    TransitionToCharacterScreen {
        rx: OwnedReadHalf,
        decrypt_data: (usize, u8),
    },
    SetSelection {
        guid: Option<SelectableGuid>,
    },
    SetTutorialFlag {
        flag: u32,
    },
    // Marks every tutorial as already shown
    ClearTutorials,
    // Marks every tutorial as not shown yet
    ResetTutorials,
    StartCombat {
        victim: LivingGuid,
    },
    StopCombat,
    ChatMessage(ChatMessage),
    SetAFK {
        message: CString,
    },
    SetDND {
        message: CString,
    },
}

fn parse_slot(slot: u8) -> Slot {
    match slot {
        0 => Slot::Equipment(EquipmentSlot::Head),
        1 => Slot::Equipment(EquipmentSlot::Neck),
        2 => Slot::Equipment(EquipmentSlot::Shoulders),
        3 => Slot::Equipment(EquipmentSlot::Body),
        4 => Slot::Equipment(EquipmentSlot::Chest),
        5 => Slot::Equipment(EquipmentSlot::Waist),
        6 => Slot::Equipment(EquipmentSlot::Legs),
        7 => Slot::Equipment(EquipmentSlot::Feet),
        8 => Slot::Equipment(EquipmentSlot::Wrists),
        9 => Slot::Equipment(EquipmentSlot::Hands),
        10 => Slot::Equipment(EquipmentSlot::Finger1),
        11 => Slot::Equipment(EquipmentSlot::Finger2),
        12 => Slot::Equipment(EquipmentSlot::Trinket1),
        13 => Slot::Equipment(EquipmentSlot::Trinket2),
        14 => Slot::Equipment(EquipmentSlot::Back),
        15 => Slot::Equipment(EquipmentSlot::MainHand),
        16 => Slot::Equipment(EquipmentSlot::OffHand),
        17 => Slot::Equipment(EquipmentSlot::Ranged),
        18 => Slot::Equipment(EquipmentSlot::Tabard),
        19..=22 => Slot::BagItem(slot - 19),
        23..=38 => Slot::MainBag(slot - 23),
        _ => todo!(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EquipmentSlot {
    Head = 0,
    Neck = 1,
    Shoulders = 2,
    Body = 3,
    Chest = 4,
    Waist = 5,
    Legs = 6,
    Feet = 7,
    Wrists = 8,
    Hands = 9,
    Finger1 = 10,
    Finger2 = 11,
    Trinket1 = 12,
    Trinket2 = 13,
    Back = 14,
    MainHand = 15,
    OffHand = 16,
    Ranged = 17,
    Tabard = 18,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Equipment(EquipmentSlot),
    BagItem(u8),
    MainBag(u8),
}

impl Slot {
    pub fn is_equipable(&self) -> bool {
        matches!(self, Slot::Equipment(_) | Slot::BagItem(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementOpcode {
    StartForward,
    StartBackward,
    Stop,
    StartStrafeLeft,
    StartStrafeRight,
    StopStrafe,
    Jump,
    StartTurnLeft,
    StartTurnRight,
    StopTurn,
    StartPitchUp,
    StartPitchDown,
    StopPitch,
    SetRunMode,
    SetWalkMode,
    FallLand,
    StartSwim,
    StopSwim,
    SetFacing,
    SetPitch,
    Heartbeat,
}

pub enum ChatMessage {
    Say {
        language: ValidLanguage,
        text: CString,
    },
    Party {
        language: ValidLanguage,
        text: CString,
    },
    Raid {
        language: ValidLanguage,
        text: CString,
    },
    Guild {
        language: ValidLanguage,
        text: CString,
    },
    Officer {
        language: ValidLanguage,
        text: CString,
    },
    Yell {
        language: ValidLanguage,
        text: CString,
    },
    Whisper {
        language: ValidLanguage,
        target_name: CString,
        text: CString,
    },
    Emote {
        language: ValidLanguage,
        text: CString,
    },
    Channel {
        language: ValidLanguage,
        channel_name: CString,
        text: CString,
    },
    RaidLeader {
        language: ValidLanguage,
        text: CString,
    },
    RaidWarning {
        language: ValidLanguage,
        text: CString,
    },
    BattleGround {
        language: ValidLanguage,
        text: CString,
    },
    BattleGroundLeader {
        language: ValidLanguage,
        text: CString,
    },
}

struct CharacterNameQueryData {
    pub name: String,
    pub race: u8,
    pub gender: u8,
    pub class: u8,
}

impl CharacterNameQueryData {
    pub async fn load_from_db(
        db: &Pool<Sqlite>,
        guid: Guid<guid::Player>,
    ) -> Result<Option<Self>, sqlx::Error> {
        let character_id = guid.get_u32();
        let model = match sqlx::query!("SELECT * FROM character WHERE id = ?", character_id)
            .fetch_one(db)
            .await
        {
            Ok(v) => v,
            Err(sqlx::Error::RowNotFound) => return Ok(None),
            Err(e) => return Err(e),
        };

        Ok(Some(Self {
            name: model.name,
            race: model.race as u8,
            gender: model.gender as u8,
            class: model.class as u8,
        }))
    }
}

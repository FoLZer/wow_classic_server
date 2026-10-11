use std::{collections::HashMap, ffi::CString, num::NonZeroU32};

use bit_vec::BitVec;
use common::guid::{self, AnyGuid, GossipSourceGuid, Guid, LivingGuid, SelectableGuid};
use gameobjects::{player::TutorialFlags, tracked_field::ClientUpdatable};
use log::{error, warn};
use packets::{
    chat_message::{ChatTag, ServerChatMessage, ServerChatType},
    gossip::{GossipMenuItem, U32Counted},
    movement_info::{MovementFlags, MovementInfo},
    server::ServerPacket,
    update_data::{
        MovementUpdate, PositionUpdate, PossibleUpdate, UpdateBlocks, UpdateData, ValuesUpdate,
    },
    vendor_query::{SerializedVendorItem, VendorQueryError, VendorQueryResult},
};
use tokio::io::AsyncWriteExt;

use crate::{
    objects::{
        character::{Character, MoveItemError},
        creature::MeleeState,
        gossip::GossipCursor,
    },
    packet_handler::{ChatMessage, Command, MovementOpcode, PlayerUpdateData, packet_handler},
    server::Server,
};

impl Server {
    pub(super) async fn process_player_updates(&mut self) {
        for update in self.player_update_queue.try_iter() {
            let character_id = update.character_id;
            match update.data {
                PlayerUpdateData::Movement {
                    opcode,
                    movement_info,
                } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };
                    character.position.0 = movement_info.pos_x;
                    character.position.1 = movement_info.pos_y;
                    character.position.2 = movement_info.pos_z;
                    character.orientation = movement_info.orientation;

                    // Relay movement to other players
                    for recipient in self.characters.values() {
                        if recipient.guid() == character_id || !recipient.sees_player(character_id)
                        {
                            continue;
                        }

                        let response = match opcode {
                            MovementOpcode::StartForward => {
                                packets::server::MSG_MOVE_START_FORWARD {
                                    mover: character_id.into(),
                                    movement_info: movement_info.clone(),
                                }
                                .to_bytes(
                                    Some(recipient.session_key),
                                    &mut *recipient.encrypt_data.lock().await,
                                )
                            }
                            MovementOpcode::StartBackward => {
                                packets::server::MSG_MOVE_START_BACKWARD {
                                    mover: character_id.into(),
                                    movement_info: movement_info.clone(),
                                }
                                .to_bytes(
                                    Some(recipient.session_key),
                                    &mut *recipient.encrypt_data.lock().await,
                                )
                            }
                            MovementOpcode::Stop => packets::server::MSG_MOVE_STOP {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::StartStrafeLeft => {
                                packets::server::MSG_MOVE_START_STRAFE_LEFT {
                                    mover: character_id.into(),
                                    movement_info: movement_info.clone(),
                                }
                                .to_bytes(
                                    Some(recipient.session_key),
                                    &mut *recipient.encrypt_data.lock().await,
                                )
                            }
                            MovementOpcode::StartStrafeRight => {
                                packets::server::MSG_MOVE_START_STRAFE_RIGHT {
                                    mover: character_id.into(),
                                    movement_info: movement_info.clone(),
                                }
                                .to_bytes(
                                    Some(recipient.session_key),
                                    &mut *recipient.encrypt_data.lock().await,
                                )
                            }
                            MovementOpcode::StopStrafe => packets::server::MSG_MOVE_STOP_STRAFE {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::Jump => packets::server::MSG_MOVE_JUMP {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::StartTurnLeft => {
                                packets::server::MSG_MOVE_START_TURN_LEFT {
                                    mover: character_id.into(),
                                    movement_info: movement_info.clone(),
                                }
                                .to_bytes(
                                    Some(recipient.session_key),
                                    &mut *recipient.encrypt_data.lock().await,
                                )
                            }
                            MovementOpcode::StartTurnRight => {
                                packets::server::MSG_MOVE_START_TURN_RIGHT {
                                    mover: character_id.into(),
                                    movement_info: movement_info.clone(),
                                }
                                .to_bytes(
                                    Some(recipient.session_key),
                                    &mut *recipient.encrypt_data.lock().await,
                                )
                            }
                            MovementOpcode::StopTurn => packets::server::MSG_MOVE_STOP_TURN {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::StartPitchUp => {
                                packets::server::MSG_MOVE_START_PITCH_UP {
                                    mover: character_id.into(),
                                    movement_info: movement_info.clone(),
                                }
                                .to_bytes(
                                    Some(recipient.session_key),
                                    &mut *recipient.encrypt_data.lock().await,
                                )
                            }
                            MovementOpcode::StartPitchDown => {
                                packets::server::MSG_MOVE_START_PITCH_DOWN {
                                    mover: character_id.into(),
                                    movement_info: movement_info.clone(),
                                }
                                .to_bytes(
                                    Some(recipient.session_key),
                                    &mut *recipient.encrypt_data.lock().await,
                                )
                            }
                            MovementOpcode::StopPitch => packets::server::MSG_MOVE_STOP_PITCH {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::SetRunMode => packets::server::MSG_MOVE_SET_RUN_MODE {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::SetWalkMode => {
                                packets::server::MSG_MOVE_SET_WALK_MODE {
                                    mover: character_id.into(),
                                    movement_info: movement_info.clone(),
                                }
                                .to_bytes(
                                    Some(recipient.session_key),
                                    &mut *recipient.encrypt_data.lock().await,
                                )
                            }
                            MovementOpcode::FallLand => packets::server::MSG_MOVE_FALL_LAND {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::StartSwim => packets::server::MSG_MOVE_START_SWIM {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::StopSwim => packets::server::MSG_MOVE_STOP_SWIM {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::SetFacing => packets::server::MSG_MOVE_SET_FACING {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::SetPitch => packets::server::MSG_MOVE_SET_PITCH {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                            MovementOpcode::Heartbeat => packets::server::MSG_MOVE_HEARTBEAT {
                                mover: character_id.into(),
                                movement_info: movement_info.clone(),
                            }
                            .to_bytes(
                                Some(recipient.session_key),
                                &mut *recipient.encrypt_data.lock().await,
                            ),
                        };

                        let mut lock = recipient.stream_tx.lock().await;
                        if let Err(e) = lock.write_all(&response).await {
                            warn!(
                                "Failed to relay movement to client (character_id: {}). Error: {:?}",
                                recipient.object_fields.guid.get().get(),
                                e
                            );
                        }
                    }
                }
                PlayerUpdateData::SwapInventoryItem { src, dst } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    let result = character
                        .swap_items(&self.game_data_accessor, src, dst)
                        .await;

                    let result = match result {
                        Ok(()) => continue,
                        Err(MoveItemError::Inventory(e)) => e,
                        Err(MoveItemError::Database(e)) => {
                            error!(
                                "Failed to swap items due to a DB error (character_id: {}). Error: {}",
                                character_id.get(),
                                e
                            );
                            continue;
                        }
                    };

                    let response = packets::server::SMSG_INVENTORY_CHANGE_FAILURE { result };

                    let mut lock = character.stream_tx.lock().await;

                    if let Err(e) = lock
                        .write_all(&response.to_bytes(
                            Some(character.session_key),
                            &mut *character.encrypt_data.lock().await,
                        ))
                        .await
                    {
                        warn!(
                            "Failed to send SMSG_INVENTORY_CHANGE_FAILURE to client (character_id: {}). Error: {:?}",
                            character_id.get(),
                            e
                        )
                    };
                }
                PlayerUpdateData::AutoEquipItem { src } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    let result = character
                        .auto_equip_item(&self.game_data_accessor, src)
                        .await;

                    let result = match result {
                        Ok(()) => continue,
                        Err(MoveItemError::Inventory(e)) => e,
                        Err(MoveItemError::Database(e)) => {
                            error!(
                                "Failed to swap items due to a DB error (character_id: {}). Error: {}",
                                character_id.get(),
                                e
                            );
                            continue;
                        }
                    };

                    let response = packets::server::SMSG_INVENTORY_CHANGE_FAILURE { result };

                    let mut lock = character.stream_tx.lock().await;

                    if let Err(e) = lock
                        .write_all(&response.to_bytes(
                            Some(character.session_key),
                            &mut *character.encrypt_data.lock().await,
                        ))
                        .await
                    {
                        warn!(
                            "Failed to send SMSG_INVENTORY_CHANGE_FAILURE to client (character_id: {}). Error: {:?}",
                            character_id.get(),
                            e
                        )
                    };
                }
                PlayerUpdateData::StoreItemInBackpack { src } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    let result = character
                        .store_item_in_backpack(&self.game_data_accessor, src)
                        .await;

                    let result = match result {
                        Ok(()) => continue,
                        Err(MoveItemError::Inventory(e)) => e,
                        Err(MoveItemError::Database(e)) => {
                            error!(
                                "Failed to store an item due to a DB error (character_id: {}). Error: {}",
                                character_id.get(),
                                e
                            );
                            continue;
                        }
                    };

                    let response = packets::server::SMSG_INVENTORY_CHANGE_FAILURE { result };

                    let mut lock = character.stream_tx.lock().await;

                    if let Err(e) = lock
                        .write_all(&response.to_bytes(
                            Some(character.session_key),
                            &mut *character.encrypt_data.lock().await,
                        ))
                        .await
                    {
                        warn!(
                            "Failed to send SMSG_INVENTORY_CHANGE_FAILURE to client (character_id: {}). Error: {:?}",
                            character_id.get(),
                            e
                        )
                    };
                }
                PlayerUpdateData::ResendSheathState => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    character.unit_fields.bytes_3.force_update();
                }
                PlayerUpdateData::SetSheathState { state } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    character
                        .unit_fields
                        .bytes_3
                        .get_mut_using_copy()
                        .set_sheath_state(state);
                }
                PlayerUpdateData::ResendAnimationState => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    character.unit_fields.bytes_2.force_update();
                }
                PlayerUpdateData::SetAnimationState { state } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    character
                        .unit_fields
                        .bytes_2
                        .get_mut_using_copy()
                        .set_stand_state(state);
                }
                PlayerUpdateData::ForceKick => {
                    self.characters.remove(&character_id);
                }
                PlayerUpdateData::TransitionToCharacterScreen { rx, decrypt_data } => {
                    let Some(character) = self.characters.remove(&character_id) else {
                        warn!(
                            "Tried to transition a missing character (character_id: {})",
                            character_id.get()
                        );
                        return;
                    };
                    // If this fails the connection is going to be shut due to Drop being called
                    let _ = self.character_transition_to_character_screen_tx.send((
                        character,
                        rx,
                        decrypt_data,
                    ));
                }
                PlayerUpdateData::SetSelection { guid } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    *character.unit_fields.target.get_mut_using_copy() = guid;

                    if let Some(melee_state) = &mut character.melee_state
                        && melee_state.victim.is_some()
                    {
                        // TODO: targeting checks

                        async fn send_new_attack_target(
                            character_id: Guid<guid::Player>,
                            victim: LivingGuid,
                            characters: &mut HashMap<Guid<guid::Player>, Character>,
                        ) {
                            let response = packets::server::SMSG_ATTACKSTART {
                                attacker: LivingGuid::Player(character_id).into(),
                                victim,
                            };

                            for character in characters.values() {
                                if !character.sees_player_including_self(character_id) {
                                    continue;
                                }

                                let mut lock = character.stream_tx.lock().await;

                                if let Err(e) = lock
                                    .write_all(&response.to_bytes(
                                        Some(character.session_key),
                                        &mut *character.encrypt_data.lock().await,
                                    ))
                                    .await
                                {
                                    warn!(
                                        "Failed to send SMSG_ATTACKSTART to client (account_id: {}). Error: {:?}",
                                        character.account_id, e
                                    );
                                    continue;
                                };
                            }
                        }

                        match guid {
                            Some(SelectableGuid::Player(guid)) => {
                                let new_victim = LivingGuid::Player(guid);
                                melee_state.victim = Some(new_victim);
                                send_new_attack_target(
                                    character_id,
                                    new_victim,
                                    &mut self.characters,
                                )
                                .await;
                            }
                            Some(SelectableGuid::Unit(guid)) => {
                                let new_victim = LivingGuid::Unit(guid);
                                melee_state.victim = Some(LivingGuid::Unit(guid));
                                send_new_attack_target(
                                    character_id,
                                    new_victim,
                                    &mut self.characters,
                                )
                                .await;
                            }
                            _ => {
                                let Some(previous_victim) = melee_state.victim.take() else {
                                    continue;
                                };

                                let response = packets::server::SMSG_ATTACKSTOP {
                                    attacker: LivingGuid::Player(character_id).into(),
                                    victim: Some(previous_victim.into()),
                                    unkn: 0,
                                };

                                self.broadcast_packet(&response, |character| {
                                    !character.sees_player_including_self(character_id)
                                })
                                .await;
                            }
                        }
                    }
                }
                PlayerUpdateData::SetTutorialFlag { flag } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    character.tutorial_flags =
                        TutorialFlags::from_bits(character.tutorial_flags.into_bits() | 1 << flag);
                }
                PlayerUpdateData::ClearTutorials => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    character.tutorial_flags =
                        TutorialFlags::from_bits((1 << TutorialFlags::COUNT) - 1);
                }
                PlayerUpdateData::ResetTutorials => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    character.tutorial_flags = TutorialFlags::new();
                }
                PlayerUpdateData::StartCombat { victim } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    //TODO: need to check victim being targetable for combat, e.g. not dead, range checks

                    match &mut character.melee_state {
                        Some(state) => state.victim = Some(victim),
                        None => {
                            character.melee_state = Some(MeleeState {
                                victim: Some(victim),
                                last_main_hand: None,
                                last_off_hand: None,
                            });
                        }
                    }

                    let response = packets::server::SMSG_ATTACKSTART {
                        attacker: LivingGuid::Player(character_id),
                        victim,
                    };

                    self.broadcast_packet(&response, |character| {
                        !character.sees_player_including_self(character_id)
                    })
                    .await;
                }
                PlayerUpdateData::StopCombat => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    let previous_victim = character
                        .melee_state
                        .as_mut()
                        .and_then(|state| state.victim.take());

                    let response = packets::server::SMSG_ATTACKSTOP {
                        attacker: LivingGuid::Player(character_id).into(),
                        victim: previous_victim.map(|v| v.into()),
                        unkn: 0,
                    };

                    self.broadcast_packet(&response, |character| {
                        !character.sees_player_including_self(character_id)
                    })
                    .await;
                }
                PlayerUpdateData::ChatMessage(message) => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    let source_position = character.position;
                    match message {
                        ChatMessage::Say { language, text } => {
                            let response = packets::server::SMSG_MESSAGECHAT {
                                chat: ServerChatMessage {
                                    ty: ServerChatType::Say {
                                        sender: character_id,
                                        lang: language.get(),
                                        tag: ChatTag::None, // TODO
                                    },
                                    text,
                                },
                            };

                            self.broadcast_packet_in_range(
                                &response,
                                source_position,
                                self.say_message_range,
                                |_| false,
                            )
                            .await;
                        }
                        ChatMessage::Party { language, text } => todo!(),
                        ChatMessage::Raid { language, text } => todo!(),
                        ChatMessage::Guild { language, text } => todo!(),
                        ChatMessage::Officer { language, text } => todo!(),
                        ChatMessage::Yell { language, text } => {
                            let response = packets::server::SMSG_MESSAGECHAT {
                                chat: ServerChatMessage {
                                    ty: ServerChatType::Yell {
                                        sender: character_id,
                                        lang: language.get(),
                                        tag: ChatTag::None, // TODO
                                    },
                                    text,
                                },
                            };

                            self.broadcast_packet_in_range(
                                &response,
                                source_position,
                                self.yell_message_range,
                                |_| false,
                            )
                            .await;
                        }
                        ChatMessage::Whisper {
                            language,
                            target_name,
                            text,
                        } => todo!(),
                        ChatMessage::Emote { language, text } => todo!(),
                        ChatMessage::Channel {
                            language,
                            channel_name,
                            text,
                        } => todo!(),
                        ChatMessage::RaidLeader { language, text } => todo!(),
                        ChatMessage::RaidWarning { language, text } => todo!(),
                        ChatMessage::BattleGround { language, text } => todo!(),
                        ChatMessage::BattleGroundLeader { language, text } => todo!(),
                    }
                }
                PlayerUpdateData::SetAFK { message } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    if message.is_empty() {
                        character
                            .player_fields
                            .flags
                            .get_mut_using_copy()
                            .set_afk(false);
                    } else {
                        character
                            .player_fields
                            .flags
                            .get_mut_using_copy()
                            .set_afk(true);
                    }
                }
                PlayerUpdateData::SetDND { message } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    if message.is_empty() {
                        character
                            .player_fields
                            .flags
                            .get_mut_using_copy()
                            .set_dnd(false);
                    } else {
                        character
                            .player_fields
                            .flags
                            .get_mut_using_copy()
                            .set_dnd(true);
                    }
                }
                PlayerUpdateData::RunCommand(command) => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };

                    match command {
                        Command::GetPosition => {
                            let response = packets::server::SMSG_MESSAGECHAT {
                                chat: ServerChatMessage {
                                    ty: ServerChatType::System,
                                    text: CString::new(format!(
                                        "Your position: (x: {}, y: {}, z: {}, orientation: {})",
                                        character.position.0,
                                        character.position.1,
                                        character.position.2,
                                        character.orientation
                                    ))
                                    .unwrap(),
                                },
                            };

                            let mut lock = character.stream_tx.lock().await;

                            if let Err(e) = lock
                                .write_all(&response.to_bytes(
                                    Some(character.session_key),
                                    &mut *character.encrypt_data.lock().await,
                                ))
                                .await
                            {
                                warn!(
                                    "Failed to send SMSG_MESSAGECHAT to client (account_id: {}). Error: {:?}",
                                    character.account_id, e
                                );
                                continue;
                            };
                        }
                    }
                }
                PlayerUpdateData::ListVendorInventory { vendor_guid } => {
                    let Some(character) = self.characters.get(&character_id) else {
                        continue;
                    };

                    let respond = async |query_result: VendorQueryResult| {
                        let response = packets::server::SMSG_LIST_INVENTORY {
                            source: vendor_guid,
                            query_result,
                        };

                        let mut lock = character.stream_tx.lock().await;

                        if let Err(e) = lock
                            .write_all(&response.to_bytes(
                                Some(character.session_key),
                                &mut *character.encrypt_data.lock().await,
                            ))
                            .await
                        {
                            warn!(
                                "Failed to send SMSG_LIST_INVENTORY to client (account_id: {}). Error: {:?}",
                                character.account_id, e
                            );
                        };
                    };

                    if !character.is_alive() {
                        respond(VendorQueryResult::Err(VendorQueryError::PlayerDead)).await;
                        continue;
                    }

                    let Some(vendor) = self.creatures.get(&vendor_guid) else {
                        respond(VendorQueryResult::Err(VendorQueryError::VendorDead)).await;
                        continue;
                    };

                    if !vendor.is_alive() {
                        respond(VendorQueryResult::Err(VendorQueryError::VendorDead)).await;
                        continue;
                    }

                    // TODO: check distance & hostility

                    let Some(inventory) = &vendor.vendor_menu else {
                        respond(VendorQueryResult::Err(VendorQueryError::NoInventory)).await;
                        continue;
                    };

                    debug_assert_ne!(inventory.len(), 0);

                    respond(VendorQueryResult::Ok {
                        items: inventory
                            .iter()
                            .enumerate()
                            .map(|(index, item)| SerializedVendorItem {
                                index: NonZeroU32::new(index as u32 + 1).unwrap(),
                                item_id: item.item_id,
                                display_id: item.display_id,
                                num_available: item.num_available,
                                price: item.price,
                                durability: item.durability,
                                purchase_amount: item.purchase_amount,
                            })
                            .collect(),
                    })
                    .await;
                }
                PlayerUpdateData::OpenGossip { source_guid } => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        return;
                    };
                    character.gossip_cursor = None;
                    if !character.is_alive() {
                        return;
                    }

                    let Some(creature) = self.creatures.get(&source_guid) else {
                        return;
                    };
                    if !creature.is_alive() {
                        return;
                    }

                    let Some(gossip_menu_root) = &creature.gossip_menu else {
                        return;
                    };

                    character.gossip_cursor = Some(GossipCursor {
                        source_guid,
                        menu_id: gossip_menu_root.root_menu_id,
                    });
                    self.send_gossip_menu(character_id).await;
                }
                PlayerUpdateData::SelectGossipOption {
                    source_guid,
                    item_index,
                } => {
                    let Some(character) = self.characters.get(&character_id) else {
                        return;
                    };
                    let Some(cursor) = character.gossip_cursor else {
                        return;
                    };
                    if cursor.source_guid != source_guid {
                        warn!(
                            "Client (character_id: {}) selected a gossip item from unexpected creature {}",
                            character_id.get(),
                            source_guid.get()
                        );
                        return;
                    }

                    let Some(creature) = self.creatures.get(&source_guid) else {
                        return;
                    };
                    if !creature.is_alive() {
                        return;
                    }
                    let Some(gossip_menu_root) = &creature.gossip_menu else {
                        return;
                    };
                    let Some(menu) = gossip_menu_root.menus.get(&cursor.menu_id) else {
                        return;
                    };
                    let Some(item) = menu.items.iter().find(|item| item.index == item_index) else {
                        warn!(
                            "Client (character_id: {}) selected missing gossip item {} from creature {}",
                            character_id.get(),
                            item_index,
                            source_guid.get()
                        );
                        return;
                    };
                    let next_menu_id = item.next_menu_id.unwrap_or(cursor.menu_id);
                    if !gossip_menu_root.menus.contains_key(&next_menu_id) {
                        return;
                    }

                    let Some(character) = self.characters.get_mut(&character_id) else {
                        return;
                    };
                    character.gossip_cursor = Some(GossipCursor {
                        source_guid,
                        menu_id: next_menu_id,
                    });
                    self.send_gossip_menu(character_id).await;
                }
            }
        }
    }

    async fn send_gossip_menu(&self, character_id: Guid<guid::Player>) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        let Some(cursor) = character.gossip_cursor else {
            return;
        };
        let Some(creature) = self.creatures.get(&cursor.source_guid) else {
            return;
        };
        let Some(gossip_menu_root) = &creature.gossip_menu else {
            return;
        };
        let Some(menu) = gossip_menu_root.menus.get(&cursor.menu_id) else {
            return;
        };

        let menu_items = menu
            .items
            .iter()
            .map(|item| GossipMenuItem {
                index: item.index,
                icon: item.icon,
                coded: u8::from(item.coded),
                text: CString::new(item.text.as_str()).unwrap(),
            })
            .collect();
        let response = packets::server::SMSG_GOSSIP_MESSAGE {
            source: GossipSourceGuid::Unit(cursor.source_guid),
            title_id: menu.title_text_id,
            menu_items: U32Counted(menu_items),
            quest_menu_items: U32Counted(vec![]),
        };

        let mut lock = character.stream_tx.lock().await;
        if let Err(error) = lock
            .write_all(&response.to_bytes(
                Some(character.session_key),
                &mut *character.encrypt_data.lock().await,
            ))
            .await
        {
            warn!(
                "Failed to send SMSG_GOSSIP_MESSAGE to client (account_id: {}). Error: {:?}",
                character.account_id, error
            );
        }
    }

    pub(super) async fn add_queued_characters(&mut self) {
        for (character, rx, decrypt_data) in self.world_transition_character_queue.try_iter() {
            let response = packets::server::SMSG_LOGIN_VERIFY_WORLD {
                map: character.map_id,
                position_x: character.position.0,
                position_y: character.position.1,
                position_z: character.position.2,
                orientation: character.orientation,
            };

            let mut lock = character.stream_tx.lock().await;

            if let Err(e) = lock
                .write_all(&response.to_bytes(
                    Some(character.session_key),
                    &mut *character.encrypt_data.lock().await,
                ))
                .await
            {
                error!(
                    "Failed to send SMSG_LOGIN_VERIFY_WORLD to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            let response = packets::server::SMSG_ACCOUNT_DATA_TIMES { unkn: [0; 32] };

            if let Err(e) = lock
                .write_all(&response.to_bytes(
                    Some(character.session_key),
                    &mut *character.encrypt_data.lock().await,
                ))
                .await
            {
                error!(
                    "Failed to send SMSG_ACCOUNT_DATA_TIMES to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            let response = packets::server::SMSG_SET_REST_START { unkn: 0 };

            if let Err(e) = lock
                .write_all(&response.to_bytes(
                    Some(character.session_key),
                    &mut *character.encrypt_data.lock().await,
                ))
                .await
            {
                error!(
                    "Failed to send SMSG_SET_REST_START to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            // TODO: bindpoints
            let response = packets::server::SMSG_BINDPOINTUPDATE {
                homebind_x: 0.0,
                homebind_y: 0.0,
                homebind_z: 0.0,
                homebind_map_id: 0,
                homebind_area_id: 0,
            };

            if let Err(e) = lock
                .write_all(&response.to_bytes(
                    Some(character.session_key),
                    &mut *character.encrypt_data.lock().await,
                ))
                .await
            {
                error!(
                    "Failed to send SMSG_BINDPOINTUPDATE to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            let response = packets::server::SMSG_TUTORIAL_FLAGS {
                tutorial_data0: character.tutorial_flags.into_bits() as u32,
                tutorial_data1: (character.tutorial_flags.into_bits() >> 32) as u32,
                tutorial_data2: 0,
                tutorial_data3: 0,
                tutorial_data4: 0,
                tutorial_data5: 0,
                tutorial_data6: 0,
                tutorial_data7: 0,
            };

            if let Err(e) = lock
                .write_all(&response.to_bytes(
                    Some(character.session_key),
                    &mut *character.encrypt_data.lock().await,
                ))
                .await
            {
                error!(
                    "Failed to send SMSG_TUTORIAL_FLAGS to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            let response = packets::server::SMSG_LOGIN_SETTIMESPEED {
                game_time: self.game_time,
                game_speed: 0.01666667,
            };

            if let Err(e) = lock
                .write_all(&response.to_bytes(
                    Some(character.session_key),
                    &mut *character.encrypt_data.lock().await,
                ))
                .await
            {
                error!(
                    "Failed to send SMSG_LOGIN_SETTIMESPEED to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            let mut mask_blocks = BitVec::new();
            let mut values_blocks = Vec::new();

            character
                .object_fields
                .write_full_update_block(&mut mask_blocks, &mut values_blocks);
            character
                .unit_fields
                .write_full_update_block(&mut mask_blocks, &mut values_blocks);
            character
                .player_fields
                .write_full_update_block(&mut mask_blocks, &mut values_blocks);

            let block = UpdateData::CreateNewObject {
                guid: AnyGuid::Player(character.object_fields.guid.get().clone()),
                movement: MovementUpdate {
                    is_self_update: true,
                    position: Some(PositionUpdate::Living {
                        movement_info: MovementInfo {
                            movement_flags: MovementFlags::new(),
                            timestamp: 0,
                            pos_x: character.position.0,
                            pos_y: character.position.1,
                            pos_z: character.position.2,
                            orientation: character.orientation,
                            on_transport_data: None,
                            swimming_pitch: None,
                            fall_time: 0,
                            falling_data: None,
                            spline_elevation: None,
                        },
                        walk_speed: 1.0,
                        run_speed: 80.0,
                        run_backwards_speed: 4.5,
                        swim_speed: 4.722222,
                        swim_backwards_speed: 2.5,
                        turn_speed: std::f32::consts::PI,
                    }),
                    high_guid: None,
                    is_update_all: true,
                    full_guid: PossibleUpdate::NoUpdate,
                    transport_time_millis: None,
                },
                values: ValuesUpdate {
                    mask_blocks,
                    values_blocks,
                },
            };

            let mut update_blocks = vec![block];
            let item_update_blocks = character.build_item_full_update_blocks();
            update_blocks.extend(item_update_blocks);

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks: update_blocks,
                },
            };

            if let Err(e) = lock
                .write_all(&response.to_bytes(
                    Some(character.session_key),
                    &mut *character.encrypt_data.lock().await,
                ))
                .await
            {
                error!(
                    "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            drop(lock);

            let character_id = character.object_fields.guid.get().clone();
            let session_key = character.session_key;
            let encrypt_data = character.encrypt_data.clone();
            let stream_tx = character.stream_tx.clone();

            self.characters
                .insert(character.object_fields.guid.get().clone(), *character);

            tokio::task::spawn(packet_handler(
                rx,
                stream_tx,
                session_key,
                decrypt_data,
                encrypt_data,
                character_id,
                self.player_update_queue.clone(),
                self.game_data_accessor.clone(),
                self.db.clone(),
            ));
        }
    }
}

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use bit_vec::BitVec;
use chrono::{DateTime, Local, TimeDelta};
use common::guid::{self, AnyGuid, Guid, LivingGuid, SelectableGuid};
use concurrent_queue::ConcurrentQueue;
use gameobjects::{
    player::TutorialFlags,
    tracked_field::ClientUpdatable,
    unit::{UnitFields, VirtualItemInfo},
};
use log::{error, warn};
use packets::{
    attacker_state::{DamageSchool, HitHand, HitInfo, SubDamage, SubDamages, VictimState},
    inventory_change_result::{InventoryChangeError, InventoryChangeResult},
    movement_info::{MovementFlags, MovementInfo},
    update_data::{
        MovementUpdate, PositionUpdate, PossibleUpdate, UpdateBlocks, UpdateData, ValuesUpdate,
    },
};
use rand::{RngExt, rng};
use tokio::{io::AsyncWriteExt, net::tcp::OwnedReadHalf, sync::mpsc};

use crate::{
    creature_spawner::{CreatureSpawnInfo, CreatureSpawner},
    game_data::GameDataAccessor,
    guid_allocator::GuidAllocator,
    objects::{
        character::Character,
        creature::{Creature, MeleeState},
        creature_prototype::CreaturePrototype,
    },
    packet_handler::{MovementOpcode, PlayerUpdate, PlayerUpdateData, packet_handler},
    sparse_set::SparseSet,
};

pub struct Server {
    pub game_time: DateTime<Local>,

    creature_spawners: SparseSet<CreatureSpawner>,
    creatures: HashMap<Guid<guid::Unit>, Creature>,
    characters: HashMap<Guid<guid::Player>, Character>,

    unit_guid_allocator: GuidAllocator<guid::Unit>,

    // A queue containing all parsed updates received from players during this tick
    player_update_queue: Arc<ConcurrentQueue<PlayerUpdate>>,
    world_transition_character_queue:
        Arc<ConcurrentQueue<(Box<Character>, OwnedReadHalf, (usize, u8))>>,
    character_transition_to_character_screen_tx:
        mpsc::UnboundedSender<(Character, OwnedReadHalf, (usize, u8))>,
    game_data_accessor: GameDataAccessor,
}

impl Server {
    pub async fn new(
        world_transition_character_queue: Arc<
            ConcurrentQueue<(Box<Character>, OwnedReadHalf, (usize, u8))>,
        >,
        game_data_accessor: GameDataAccessor,
        character_transition_to_character_screen_tx: mpsc::UnboundedSender<(
            Character,
            OwnedReadHalf,
            (usize, u8),
        )>,
    ) -> Result<Self, sqlx::Error> {
        let mut unit_guid_allocator = GuidAllocator::new();
        let mut creatures = HashMap::new();

        let mut creature_spawners =
            SparseSet::from_vec(game_data_accessor.load_creature_spawners().await?);

        let mut to_deactivate = Vec::new();
        for (index, spawner) in creature_spawners.iter().enumerate() {
            let (spawn_infos, keep_active) = spawner.force_first_spawn();

            for spawn_info in spawn_infos {
                // TODO: prefetch, a tick should not be stuck waiting for db access here, especially sequentially like that
                let prototype = match game_data_accessor
                    .get_creature_prototype(spawn_info.spawn_creature_id)
                    .await
                {
                    Ok(Some(v)) => v,
                    Ok(None) => {
                        error!(
                            "Tried to spawn a creature with a missing prototype (id: {})",
                            spawn_info.spawn_creature_id
                        );
                        continue;
                    }
                    Err(e) => {
                        error!(
                            "Failed to query creature prototype due to a DB error (id: {}). Error: {}",
                            spawn_info.spawn_creature_id, e
                        );
                        continue;
                    }
                };

                Self::spawn_creature(
                    &mut unit_guid_allocator,
                    prototype,
                    spawn_info,
                    &mut creatures,
                );
            }

            if !keep_active {
                to_deactivate.push(index);
            }
        }
        for index in to_deactivate {
            creature_spawners.deactivate(index);
        }

        Ok(Self {
            game_time: Local::now(),

            creature_spawners,

            creatures,
            characters: HashMap::new(),

            unit_guid_allocator,

            player_update_queue: Arc::new(ConcurrentQueue::unbounded()),
            world_transition_character_queue,
            character_transition_to_character_screen_tx,
            game_data_accessor,
        })
    }

    pub async fn update(&mut self, diff: TimeDelta) {
        let now = Instant::now();
        self.add_queued_characters().await;
        self.process_player_updates().await;

        self.process_queued_creature_spawners().await;

        // Update order: Remove -> Update -> Create
        // TODO: merge 3 packets into 1 by merging the blocks from each function
        self.remove_invisible_players_for_other_players().await;
        self.send_player_updates_to_players().await;
        self.create_new_players_for_players().await;

        self.remove_invisible_creatures_for_players().await;
        self.send_creature_updates_to_players().await;
        self.create_new_creatures_for_players().await;

        self.update_player_melee_attacks(now).await;
        self.update_creature_melee_attacks(now).await;
    }

    async fn process_player_updates(&mut self) {
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

                    let mut src_slot = match src {
                        crate::packet_handler::Slot::MainBag(slot) => {
                            character.player_fields.main_backpack_slots[slot as usize]
                                .get_mut_using_copy()
                        }
                    };

                    let Some(item) = src_slot.take() else {
                        let response = packets::server::SMSG_INVENTORY_CHANGE_FAILURE {
                            result: InventoryChangeResult::OtherError {
                                error: InventoryChangeError::SlotIsEmpty,
                                item1: None,
                                item2: None,
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
                                "Failed to send SMSG_INVENTORY_CHANGE_FAILURE to client (character_id: {}). Error: {:?}",
                                character_id.get(),
                                e
                            )
                        };

                        continue;
                    };

                    drop(src_slot);

                    let mut dst_slot = match dst {
                        crate::packet_handler::Slot::MainBag(slot) => {
                            character.player_fields.main_backpack_slots[slot as usize]
                                .get_mut_using_copy()
                        }
                    };

                    if let Some(item) = dst_slot.replace(item) {
                        drop(dst_slot);

                        let mut src_slot = match src {
                            crate::packet_handler::Slot::MainBag(slot) => {
                                character.player_fields.main_backpack_slots[slot as usize]
                                    .get_mut_using_copy()
                            }
                        };

                        src_slot.replace(item);
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

                                for character in self.characters.values() {
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
                                            "Failed to send SMSG_ATTACKSTOP to client (account_id: {}). Error: {:?}",
                                            character.account_id, e
                                        );
                                        continue;
                                    };
                                }
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

                    for character in self.characters.values() {
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

                    for character in self.characters.values() {
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
                                "Failed to send SMSG_ATTACKSTOP to client (account_id: {}). Error: {:?}",
                                character.account_id, e
                            );
                            continue;
                        };
                    }
                }
            }
        }
    }

    async fn add_queued_characters(&mut self) {
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
            ));
        }
    }

    fn is_character_visible(&self, from: &Character, to: &Character) -> bool {
        true // TODO
    }

    fn is_creature_visible_to_player(&self, from: &Character, to: &Creature) -> bool {
        let dist = (from.position.0 - to.position.0).powi(2)
            + (from.position.1 - to.position.1).powi(2)
            + (from.position.2 - to.position.2).powi(2);

        dist < 20000.0
        // TODO
    }

    /* --- Combat --- */

    async fn update_player_melee_attacks(&mut self, now: Instant) {
        let mut swings = Vec::new();

        for (guid, character) in &mut self.characters {
            let Some(melee_state) = &mut character.melee_state else {
                continue;
            };
            let Some(victim) = melee_state.victim else {
                continue;
            };

            // TODO: check that the player can attack (not dead, stunned, pacified, casting, etc.)
            // TODO: range check, send SMSG_ATTACKSWING_NOTINRANGE without consuming the swing
            // TODO: facing check, send SMSG_ATTACKSWING_BADFACING without consuming the swing

            let main_attack_time =
                Duration::from_millis(*character.unit_fields.base_attack_time.get() as u64);
            if melee_state
                .last_main_hand
                .is_none_or(|last| now.duration_since(last) >= main_attack_time)
            {
                melee_state.last_main_hand = Some(now);
                swings.push((LivingGuid::Player(*guid), victim, HitHand::Main));
            }

            /* Requires a check that offhand item is present
            let main_attack_time =
                Duration::from_millis(*character.unit_fields.offhand_attack_time.get() as u64);
            if melee_state
                .last_off_hand
                .is_none_or(|last| now.duration_since(last) >= main_attack_time)
            {
                melee_state.last_off_hand = Some(now);
                swings.push((LivingGuid::Player(*guid), victim, HitHand::Off));
            }
            */
        }

        for (attacker, victim, hand) in swings {
            self.resolve_melee_swing(attacker, victim, hand).await;
        }
    }

    async fn update_creature_melee_attacks(&mut self, now: Instant) {
        let mut swings = Vec::new();

        for (guid, creature) in &mut self.creatures {
            let Some(melee_state) = &mut creature.melee_state else {
                continue;
            };
            let Some(victim) = melee_state.victim else {
                continue;
            };

            // TODO: check that the creature can attack (not dead, stunned, evading, casting, etc.)
            // TODO: range check, the creature should chase instead of swinging
            // TODO: facing check, the creature should turn to the victim instead of swinging

            let main_attack_time =
                Duration::from_millis(*creature.unit_fields.base_attack_time.get() as u64);
            if melee_state
                .last_main_hand
                .is_none_or(|last| now.duration_since(last) >= main_attack_time)
            {
                melee_state.last_main_hand = Some(now);
                swings.push((LivingGuid::Unit(*guid), victim, HitHand::Main));
            }

            /* Requires a check that offhand item is present
            let main_attack_time =
                Duration::from_millis(*creature.unit_fields.offhand_attack_time.get() as u64);
            if melee_state
                .last_off_hand
                .is_none_or(|last| now.duration_since(last) >= main_attack_time)
            {
                melee_state.last_off_hand = Some(now);
                swings.push((LivingGuid::Unit(*guid), victim, HitHand::Off));
            }
            */
        }

        for (attacker, victim, hand) in swings {
            self.resolve_melee_swing(attacker, victim, hand).await;
        }
    }

    async fn resolve_melee_swing(
        &mut self,
        attacker: LivingGuid,
        victim: LivingGuid,
        hand: HitHand,
    ) {
        // An earlier swing this tick could have killed the attacker
        if self
            .living_unit_fields(attacker)
            .is_none_or(|fields| *fields.health.get() == 0)
        {
            return;
        }

        let Some(victim_fields) = self.living_unit_fields_mut(victim) else {
            // TODO: victim is gone (logged out, despawned), stop the attack with SMSG_ATTACKSTOP
            return;
        };

        if *victim_fields.health.get() == 0 {
            // TODO: SMSG_ATTACKSWING_DEADTARGET for players, drop the victim for creatures
            return;
        }

        // TODO: check that the victim can be attacked (not immune, not evading, faction, etc.)

        // TODO: roll the hit table (miss/dodge/parry/block/crit/glancing/crushing) and calculate the damage
        let damage = 1;
        let hit_info = HitInfo::new()
            .with_affects_victim(true)
            .with_off_hand(matches!(hand, HitHand::Off));
        let victim_state = VictimState::Normal;

        let health = *victim_fields.health.get();
        *victim_fields.health.get_mut_using_copy() = health.saturating_sub(damage);
        // TODO: death handling once health reaches 0

        let response = packets::server::SMSG_ATTACKERSTATEUPDATE {
            hit_info,
            attacker: attacker.into(),
            victim: victim.into(),
            total_damage: damage,
            sub_damages: SubDamages(vec![SubDamage {
                school: DamageSchool::Physical,
                damage: damage as f32,
                int_damage: damage,
                absorbed: 0,
                resisted: 0,
            }]),
            victim_state,
            unkn: 0,
            spell_id: None,
            blocked: 0,
        };

        for character in self.characters.values() {
            if !character.sees_including_self(attacker) && !character.sees_including_self(victim) {
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
                    "Failed to send SMSG_ATTACKERSTATEUPDATE to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                continue;
            };
        }
    }

    fn living_unit_fields(&self, guid: LivingGuid) -> Option<&UnitFields> {
        match guid {
            LivingGuid::Player(guid) => self.characters.get(&guid).map(|v| &v.unit_fields),
            LivingGuid::Unit(guid) => self.creatures.get(&guid).map(|v| &v.unit_fields),
        }
    }

    fn living_unit_fields_mut(&mut self, guid: LivingGuid) -> Option<&mut UnitFields> {
        match guid {
            LivingGuid::Player(guid) => self.characters.get_mut(&guid).map(|v| &mut v.unit_fields),
            LivingGuid::Unit(guid) => self.creatures.get_mut(&guid).map(|v| &mut v.unit_fields),
        }
    }

    /* --- Visibility --- */

    // Players
    async fn send_player_updates_to_players(&mut self) {
        let mut update_blocks = HashMap::new();

        for character in self.characters.values_mut() {
            let mut mask_blocks = BitVec::new();
            let mut values_blocks = Vec::new();

            character
                .object_fields
                .write_update_block(&mut mask_blocks, &mut values_blocks);
            character
                .unit_fields
                .write_update_block(&mut mask_blocks, &mut values_blocks);
            character
                .player_fields
                .write_update_block(&mut mask_blocks, &mut values_blocks);

            if values_blocks.is_empty() {
                continue;
            }

            character.object_fields.clear_update_flags();
            character.unit_fields.clear_update_flags();
            character.player_fields.clear_update_flags();

            let block = UpdateData::UpdateObject {
                guid: AnyGuid::Player(character.object_fields.guid.get().clone()),
                values: ValuesUpdate {
                    mask_blocks,
                    values_blocks,
                },
            };

            update_blocks.insert(character.object_fields.guid.get().clone(), block);
        }

        for character in self.characters.values_mut() {
            let mut blocks = Vec::new();

            for (guid, block) in &update_blocks {
                if character.sees_player_including_self(*guid) {
                    blocks.push(block.clone());
                }
            }

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks,
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
                error!(
                    "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };
        }
    }

    async fn remove_invisible_players_for_other_players(&mut self) {
        let mut guids_to_remove_for_character = HashMap::new();

        for character in self.characters.values() {
            let mut guids_to_remove = Vec::new();

            for guid in &character.other_visible_players {
                let Some(other_character) = self.characters.get(guid) else {
                    warn!(
                        "Character got removed but was not cleaned up, removing. (character still in view of other characters, from: {}, to: {})",
                        character.object_fields.guid.get().get_u32(),
                        guid.get_u32()
                    );
                    guids_to_remove.push(*guid);
                    continue;
                };

                if !self.is_character_visible(&character, other_character) {
                    guids_to_remove.push(*guid);
                }
            }

            if !guids_to_remove.is_empty() {
                guids_to_remove_for_character
                    .insert(*character.object_fields.guid.get(), guids_to_remove);
            }
        }

        for (guid, guids_to_remove) in guids_to_remove_for_character {
            let Some(character) = self.characters.get_mut(&guid) else {
                // Not possible due to how this map got constructed above
                unreachable!();
            };

            let block = UpdateData::OutOfRangeDestroyObject {
                guids: guids_to_remove
                    .iter()
                    .map(|v| AnyGuid::Player(*v))
                    .collect(),
            };

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks: vec![block],
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
                error!(
                    "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            character
                .other_visible_players
                .retain(|v| !guids_to_remove.contains(v));
        }
    }

    async fn create_new_players_for_players(&mut self) {
        let mut new_visible_characters_for_characters = HashMap::new();
        for (guid, character) in &self.characters {
            let mut new_visible_characters = Vec::new();

            for (guid, potentially_new_character) in &self.characters {
                if !character.sees_player(*guid)
                    && self.is_character_visible(character, potentially_new_character)
                {
                    new_visible_characters.push(*guid);
                }
            }

            new_visible_characters_for_characters.insert(*guid, new_visible_characters);
        }

        // Potential for a cache here but I don't think that many people are going to become visible in a single tick
        // It is a possibility but will depend on is_character_visible() function being split on maps or not

        for (guid, new_visible_characters) in new_visible_characters_for_characters {
            let mut blocks = Vec::new();

            for guid in &new_visible_characters {
                let Some(character) = self.characters.get_mut(&guid) else {
                    // Not possible due to how this vec got constructed above
                    unreachable!();
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

                blocks.push(UpdateData::CreateNewObject {
                    guid: AnyGuid::Player(character.object_fields.guid.get().clone()),
                    movement: MovementUpdate {
                        is_self_update: false,
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
                });
            }

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks,
                },
            };

            let Some(character) = self.characters.get_mut(&guid) else {
                // Not possible due to how this map got constructed above
                unreachable!();
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
                    "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            for guid in new_visible_characters {
                character.other_visible_players.insert(guid);
            }
        }
    }

    // Creatures
    async fn send_creature_updates_to_players(&mut self) {
        let mut update_blocks = HashMap::new();

        for creature in self.creatures.values_mut() {
            let mut mask_blocks = BitVec::new();
            let mut values_blocks = Vec::new();

            creature
                .object_fields
                .write_update_block(&mut mask_blocks, &mut values_blocks);
            creature
                .unit_fields
                .write_update_block(&mut mask_blocks, &mut values_blocks);

            if values_blocks.is_empty() {
                continue;
            }

            creature.object_fields.clear_update_flags();
            creature.unit_fields.clear_update_flags();

            let block = UpdateData::UpdateObject {
                guid: AnyGuid::Unit(creature.object_fields.guid.get().clone()),
                values: ValuesUpdate {
                    mask_blocks,
                    values_blocks,
                },
            };

            update_blocks.insert(creature.object_fields.guid.get().clone(), block);
        }

        for character in self.characters.values_mut() {
            let mut blocks = Vec::new();

            for (guid, block) in &update_blocks {
                if character.sees_creature(*guid) {
                    blocks.push(block.clone());
                }
            }

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks,
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
                error!(
                    "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };
        }
    }

    async fn remove_invisible_creatures_for_players(&mut self) {
        let mut guids_to_remove_for_character = HashMap::new();

        for character in self.characters.values() {
            let mut guids_to_remove = Vec::new();

            for guid in &character.visible_creatures {
                let Some(other_character) = self.creatures.get(guid) else {
                    warn!(
                        "Character got removed but was not cleaned up, removing. (character still in view of other characters, from: {}, to: {})",
                        character.object_fields.guid.get().get_u32(),
                        guid.get_u32()
                    );
                    guids_to_remove.push(*guid);
                    continue;
                };

                if !self.is_creature_visible_to_player(&character, other_character) {
                    guids_to_remove.push(*guid);
                }
            }

            if !guids_to_remove.is_empty() {
                guids_to_remove_for_character
                    .insert(*character.object_fields.guid.get(), guids_to_remove);
            }
        }

        for (guid, guids_to_remove) in guids_to_remove_for_character {
            let Some(character) = self.characters.get_mut(&guid) else {
                // Not possible due to how this map got constructed above
                unreachable!();
            };

            let block = UpdateData::OutOfRangeDestroyObject {
                guids: guids_to_remove.iter().map(|v| AnyGuid::Unit(*v)).collect(),
            };

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks: vec![block],
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
                error!(
                    "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            character
                .visible_creatures
                .retain(|v| !guids_to_remove.contains(v));
        }
    }

    async fn create_new_creatures_for_players(&mut self) {
        let mut new_visible_creatures_for_characters = HashMap::new();
        for (guid, character) in &self.characters {
            let mut new_visible_characters = Vec::new();

            for (guid, potentially_new_creature) in &self.creatures {
                if !character.sees_creature(*guid)
                    && self.is_creature_visible_to_player(character, potentially_new_creature)
                {
                    new_visible_characters.push(*guid);
                }
            }

            new_visible_creatures_for_characters.insert(*guid, new_visible_characters);
        }

        // Potential for a cache here but I don't think that many people are going to become visible in a single tick
        // It is a possibility but will depend on is_character_visible() function being split on maps or not

        for (guid, new_visible_creatures) in new_visible_creatures_for_characters {
            let mut blocks = Vec::new();

            for guid in &new_visible_creatures {
                let Some(creature) = self.creatures.get_mut(&guid) else {
                    // Not possible due to how this vec got constructed above
                    unreachable!();
                };

                let mut mask_blocks = BitVec::new();
                let mut values_blocks = Vec::new();

                creature
                    .object_fields
                    .write_full_update_block(&mut mask_blocks, &mut values_blocks);
                creature
                    .unit_fields
                    .write_full_update_block(&mut mask_blocks, &mut values_blocks);

                blocks.push(UpdateData::CreateNewObject {
                    guid: AnyGuid::Unit(creature.object_fields.guid.get().clone()),
                    movement: MovementUpdate {
                        is_self_update: false,
                        position: Some(PositionUpdate::Living {
                            movement_info: MovementInfo {
                                movement_flags: MovementFlags::new(),
                                timestamp: 0,
                                pos_x: creature.position.0,
                                pos_y: creature.position.1,
                                pos_z: creature.position.2,
                                orientation: creature.orientation,
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
                });
            }

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks,
                },
            };

            let Some(character) = self.characters.get_mut(&guid) else {
                // Not possible due to how this map got constructed above
                unreachable!();
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
                    "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            for guid in new_visible_creatures {
                character.visible_creatures.insert(guid);
            }
        }
    }

    /* --- Visibility End --- */

    fn notify_creature_spawner(&mut self, spawner_index: usize) {
        let Some(spawner) = self.creature_spawners.get_mut(spawner_index) else {
            return;
        };
        spawner.notify_creature_died();
        self.creature_spawners.activate(spawner_index);
    }

    async fn process_queued_creature_spawners(&mut self) {
        let mut to_deactivate = Vec::new();
        for (index, spawner) in self.creature_spawners.iter().enumerate() {
            let (spawn_infos, keep_active) = spawner.get_creatures_to_spawn();

            for spawn_info in spawn_infos {
                // TODO: prefetch, a tick should not be stuck waiting for db access here, especially sequentially like that
                let prototype = match self
                    .game_data_accessor
                    .get_creature_prototype(spawn_info.spawn_creature_id)
                    .await
                {
                    Ok(Some(v)) => v,
                    Ok(None) => {
                        error!(
                            "Tried to spawn a creature with a missing prototype (id: {})",
                            spawn_info.spawn_creature_id
                        );
                        continue;
                    }
                    Err(e) => {
                        error!(
                            "Failed to query creature prototype due to a DB error (id: {}). Error: {}",
                            spawn_info.spawn_creature_id, e
                        );
                        continue;
                    }
                };

                Self::spawn_creature(
                    &mut self.unit_guid_allocator,
                    prototype,
                    spawn_info,
                    &mut self.creatures,
                );
            }

            if !keep_active {
                to_deactivate.push(index);
            }
        }
        for index in to_deactivate {
            self.creature_spawners.deactivate(index);
        }
    }

    fn spawn_creature(
        unit_guid_allocator: &mut GuidAllocator<guid::Unit>,
        prototype: CreaturePrototype,
        spawn_info: CreatureSpawnInfo,
        creatures_out: &mut HashMap<Guid<guid::Unit>, Creature>,
    ) {
        let guid = unit_guid_allocator
            .allocate()
            .expect("No more available guids left");

        creatures_out.insert(
            guid,
            Creature {
                position: spawn_info.position,
                orientation: spawn_info.orientation,
                object_fields: gameobjects::object::ObjectFields {
                    guid: guid.into(),
                    object_type: gameobjects::object::TypeBitField::new()
                        .with_unit(true)
                        .with_object(true)
                        .into(),
                    entry: spawn_info.spawn_creature_id.into(),
                    scale_x: 1.0.into(),
                    _padding: 0.into(),
                },
                unit_fields: gameobjects::unit::UnitFields {
                    charm: None.into(),
                    summon: None.into(),
                    charmed_by: None.into(),
                    summoned_by: None.into(),
                    created_by: None.into(),
                    target: None.into(),
                    persuaded: None.into(),
                    channel_object: None.into(),
                    health: (prototype.health.unwrap_or(prototype.max_health)).into(),
                    powers: [0.into(); 5],
                    max_health: prototype.max_health.into(),
                    max_powers: [0.into(); 5],
                    level: (rng().random_range(prototype.level)).into(),
                    faction_template: prototype.faction.into(),
                    bytes_1: gameobjects::unit::UnitFieldBytes1::new()
                        .with_race(prototype.race)
                        .with_class(prototype.class)
                        .with_gender(prototype.gender)
                        .with_power(prototype.power)
                        .into(),
                    virtual_item_slot_displays: [
                        prototype
                            .equipment_mainhand
                            .as_ref()
                            .map(|v| v.display_info_id)
                            .unwrap_or(0)
                            .into(),
                        prototype
                            .equipment_offhand
                            .as_ref()
                            .map(|v| v.display_info_id)
                            .unwrap_or(0)
                            .into(),
                        prototype
                            .equipment_ranged
                            .as_ref()
                            .map(|v| v.display_info_id)
                            .unwrap_or(0)
                            .into(),
                    ],
                    virtual_item_infos: [
                        prototype
                            .equipment_mainhand
                            .map(|v| {
                                VirtualItemInfo::new()
                                    .with_class(v.class as u8)
                                    .with_sub_class(v.sub_class as u8)
                                    .with_material(v.material as u8)
                                    .with_inventory_type(v.inventory_type)
                                    .with_sheath(v.sheath as u8)
                            })
                            .unwrap_or_default()
                            .into(),
                        prototype
                            .equipment_offhand
                            .map(|v| {
                                VirtualItemInfo::new()
                                    .with_class(v.class as u8)
                                    .with_sub_class(v.sub_class as u8)
                                    .with_material(v.material as u8)
                                    .with_inventory_type(v.inventory_type)
                                    .with_sheath(v.sheath as u8)
                            })
                            .unwrap_or_default()
                            .into(),
                        prototype
                            .equipment_ranged
                            .map(|v| {
                                VirtualItemInfo::new()
                                    .with_class(v.class as u8)
                                    .with_sub_class(v.sub_class as u8)
                                    .with_material(v.material as u8)
                                    .with_inventory_type(v.inventory_type)
                                    .with_sheath(v.sheath as u8)
                            })
                            .unwrap_or_default()
                            .into(),
                    ],
                    flags: gameobjects::unit::UnitFlags::new().into(),
                    aura: [0.into(); 48],
                    aura_flags: [0.into(); 6],
                    aura_levels: [0.into(); 12],
                    aura_applications: [0.into(); 12],
                    aura_state: 0.into(),
                    base_attack_time: prototype.base_attack_time.into(),
                    offhand_attack_time: prototype.offhand_attack_time.into(),
                    ranged_attack_time: prototype.ranged_attack_time.into(),
                    bounding_radius: 4.into(),
                    combat_reach: prototype.combat_reach.into(),
                    display_id: prototype.display_id.into(),
                    native_display_id: prototype.native_display_id.into(),
                    mount_display_id: prototype.initial_mount_display_id.unwrap_or(0).into(),
                    min_damage: 0.into(),
                    max_damage: 0.into(),
                    min_offhand_damage: 0.into(),
                    max_offhand_damage: 0.into(),
                    bytes_2: gameobjects::unit::UnitFieldBytes2::new()
                        .with_stand_state(gameobjects::unit::StandStateType::Stand)
                        .with_loyalty_level(0)
                        .with_free_talent_points(0)
                        .with_flags(gameobjects::unit::UnitFieldBytes2Flags::new())
                        .into(),
                    pet_number: 0.into(),
                    pet_name_timestamp: 0.into(),
                    pet_experience: 0.into(),
                    pet_next_level_exp: 0.into(),
                    dynamic_flags: 0.into(),
                    channel_spell: 0.into(),
                    mod_cast_speed: 1.into(),
                    created_by_spell: 0.into(),
                    npc_flags: 0.into(),
                    npc_emote_state: 0.into(),
                    training_points: 0.into(),
                    strength: 1.into(),
                    agility: 1.into(),
                    stamina: 1.into(),
                    intellect: 1.into(),
                    spirit: 1.into(),
                    normal_resistance: 0.into(),
                    holy_resistance: 0.into(),
                    fire_resistance: 0.into(),
                    nature_resistance: 0.into(),
                    frost_resistance: 0.into(),
                    shadow_resistance: 0.into(),
                    arcane_resistance: 0.into(),
                    base_mana: 100.into(),
                    base_health: 100.into(),
                    bytes_3: gameobjects::unit::UnitFieldBytes3::new()
                        .with_sheath_state(gameobjects::unit::SheathState::Unarmed)
                        .with_flags(gameobjects::unit::UnitFieldBytes3Flags::new())
                        .into(),
                    attack_power: 1.into(),
                    attack_power_mods: 0.into(),
                    attack_power_multiplier: 1.into(),
                    ranged_attack_power: 0.into(),
                    ranged_attack_power_mods: 0.into(),
                    ranged_attack_power_multiplier: 0.into(),
                    min_ranged_damage: 0.into(),
                    max_ranged_damage: 0.into(),
                    power_cost_modifiers: [0.into(); 7],
                    power_cost_multipliers: [1.into(); 7],
                    _padding: 0.into(),
                },
                melee_state: None,
            },
        );
    }
}

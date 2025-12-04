use std::{collections::HashMap, sync::Arc};

use bit_vec::BitVec;
use chrono::{DateTime, Local, TimeDelta};
use common::guid::{self, AnyGuid, Guid};
use concurrent_queue::ConcurrentQueue;
use gameobjects::tracked_field::ClientUpdatable;
use log::{error, warn};
use packets::{
    inventory_change_result::{InventoryChangeError, InventoryChangeResult},
    movement_info::{MovementFlags, MovementInfo},
    update_data::{
        MovementUpdate, PositionUpdate, PossibleUpdate, UpdateBlocks, UpdateData, ValuesUpdate,
    },
};
use tokio::{io::AsyncWriteExt, net::tcp::OwnedReadHalf};

use crate::{
    game_data::GameDataAccessor,
    objects::{character::Character, creature::Creature},
    packet_handler::{PlayerUpdate, PlayerUpdateData, packet_handler},
};

pub struct Server {
    pub game_time: DateTime<Local>,

    creatures: HashMap<Guid<guid::Unit>, Creature>,
    characters: HashMap<Guid<guid::Player>, Character>,

    // A queue containing all parsed updates received from players during this tick
    player_update_queue: Arc<ConcurrentQueue<PlayerUpdate>>,
    world_transition_character_queue: Arc<ConcurrentQueue<(Box<Character>, OwnedReadHalf)>>,
    game_data_accessor: GameDataAccessor,
}

impl Server {
    pub fn new(
        world_transition_character_queue: Arc<ConcurrentQueue<(Box<Character>, OwnedReadHalf)>>,
        game_data_accessor: GameDataAccessor,
    ) -> Self {
        let mut creatures = HashMap::new();

        // TEMP: testing creatures
        creatures.insert(
            Guid::from_u32(std::num::NonZeroU32::new(5).unwrap()),
            Creature {
                position: (-8949.95, -132.493, 83.5312),
                orientation: 0.0,
                object_fields: gameobjects::object::ObjectFields {
                    guid: Guid::from_u32(std::num::NonZeroU32::new(5).unwrap()).into(),
                    object_type: gameobjects::object::TypeBitField::new()
                        .with_unit(true)
                        .with_object(true)
                        .into(),
                    entry: 6.into(),
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
                    health: 100.into(),
                    powers: [0.into(); 5],
                    max_health: 500.into(),
                    max_powers: [0.into(); 5],
                    level: 1.into(),
                    faction_template: 25.into(),
                    bytes_1: gameobjects::unit::UnitFieldBytes1::new()
                        .with_class(1)
                        .with_gender(1)
                        .into(),
                    virtual_item_slot_displays: [0.into(); 3],
                    virtual_item_infos: [0.into(); 6],
                    flags: gameobjects::unit::UnitFlags::new().into(),
                    aura: [0.into(); 48],
                    aura_flags: [0.into(); 6],
                    aura_levels: [0.into(); 12],
                    aura_applications: [0.into(); 12],
                    aura_state: 0.into(),
                    base_attack_time: 1.into(),
                    offhand_attack_time: 2.into(),
                    ranged_attack_time: 3.into(),
                    bounding_radius: 4.into(),
                    combat_reach: 5.into(),
                    display_id: 10913.into(),
                    native_display_id: 10913.into(),
                    mount_display_id: 0.into(),
                    min_damage: 70.into(),
                    max_damage: 80.into(),
                    min_offhand_damage: 90.into(),
                    max_offhand_damage: 130.into(),
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
            },
        );

        Self {
            game_time: Local::now(),

            creatures,
            characters: HashMap::new(),

            player_update_queue: Arc::new(ConcurrentQueue::unbounded()),
            world_transition_character_queue,
            game_data_accessor,
        }
    }

    pub async fn update(&mut self, diff: TimeDelta) {
        self.add_queued_characters().await;
        self.process_player_updates().await;

        // Update order: Remove -> Update -> Create
        // TODO: merge 3 packets into 1 by merging the blocks from each function
        self.remove_invisible_players_for_other_players().await;
        self.send_player_updates_to_players().await;
        self.create_new_players_for_players().await;

        self.remove_invisible_creatures_for_players().await;
        self.send_creature_updates_to_players().await;
        self.create_new_creatures_for_players().await;
    }

    async fn process_player_updates(&mut self) {
        for update in self.player_update_queue.try_iter() {
            let character_id = update.character_id;
            match update.data {
                PlayerUpdateData::Movement(movement_info) => {
                    let Some(character) = self.characters.get_mut(&character_id) else {
                        continue;
                    };
                    character.position.0 = movement_info.pos_x;
                    character.position.1 = movement_info.pos_y;
                    character.position.2 = movement_info.pos_z;

                    //TODO
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
            }
        }
    }

    async fn add_queued_characters(&mut self) {
        for (character, rx) in self.world_transition_character_queue.try_iter() {
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

            //TODO: tutorial
            let response = packets::server::SMSG_TUTORIAL_FLAGS {
                tutorial_data0: 0,
                tutorial_data1: 0,
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
                            fall_time: Some(0),
                            falling_data: None,
                            spline_elevation: None,
                        },
                        walk_speed: 1.0,
                        run_speed: 80.0,
                        run_backwards_speed: 4.5,
                        swim_speed: 0.0,
                        swim_backwards_speed: 0.0,
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
            let decrypt_data = character.decrypt_data;
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
                if character.other_visible_players.contains(guid)
                    || character.object_fields.guid == *guid
                {
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
                if !character.other_visible_players.contains(guid)
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
                                fall_time: Some(0),
                                falling_data: None,
                                spline_elevation: None,
                            },
                            walk_speed: 1.0,
                            run_speed: 80.0,
                            run_backwards_speed: 4.5,
                            swim_speed: 0.0,
                            swim_backwards_speed: 0.0,
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
                if character.visible_creatures.contains(guid) {
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
                if !character.visible_creatures.contains(guid)
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
                                fall_time: Some(0),
                                falling_data: None,
                                spline_elevation: None,
                            },
                            walk_speed: 1.0,
                            run_speed: 80.0,
                            run_backwards_speed: 4.5,
                            swim_speed: 0.0,
                            swim_backwards_speed: 0.0,
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
}

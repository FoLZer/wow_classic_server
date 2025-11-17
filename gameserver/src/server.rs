use std::{collections::HashMap, sync::Arc};

use bit_vec::BitVec;
use chrono::{DateTime, Local, TimeDelta};
use common::guid::{self, AnyGuid, Guid, GuidType};
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
    character::Character,
    game_data::GameDataAccessor,
    packet_handler::{PlayerUpdate, PlayerUpdateData, packet_handler},
};

pub struct Server {
    pub game_time: DateTime<Local>,

    characters: HashMap<Guid<guid::Player>, Character>,

    // A queue containing all parsed updates received from players during this tick
    player_update_queue: Arc<ConcurrentQueue<PlayerUpdate>>,
    world_transition_character_queue: Arc<ConcurrentQueue<(Character, OwnedReadHalf)>>,
    game_data_accessor: GameDataAccessor,
}

impl Server {
    pub fn new(
        world_transition_character_queue: Arc<ConcurrentQueue<(Character, OwnedReadHalf)>>,
        game_data_accessor: GameDataAccessor,
    ) -> Self {
        Self {
            game_time: Local::now(),

            characters: HashMap::new(),

            player_update_queue: Arc::new(ConcurrentQueue::unbounded()),
            world_transition_character_queue,
            game_data_accessor,
        }
    }

    pub async fn update(&mut self, diff: TimeDelta) {
        self.add_queued_characters().await;
        self.process_player_updates().await;

        self.send_updates_to_players().await;
    }

    async fn process_player_updates(&mut self) {
        for update in self.player_update_queue.try_iter() {
            let character_id = update.character_id;
            match update.data {
                PlayerUpdateData::Movement(movement_info) => (),
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
                            .write_all(&response.to_bytes(Some(character.session_key)))
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
                .write_all(&response.to_bytes(Some(character.session_key)))
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
                .write_all(&response.to_bytes(Some(character.session_key)))
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
                .write_all(&response.to_bytes(Some(character.session_key)))
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
                .write_all(&response.to_bytes(Some(character.session_key)))
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
                .write_all(&response.to_bytes(Some(character.session_key)))
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
                .write_all(&response.to_bytes(Some(character.session_key)))
                .await
            {
                error!(
                    "Failed to send SMSG_LOGIN_SETTIMESPEED to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            let test_block = {
                let guid = Guid::from_u32(std::num::NonZeroU32::new(2).unwrap());

                let mut mask_blocks = BitVec::new();
                let mut values_blocks = Vec::new();

                (gameobjects::object::ObjectFields {
                    guid: guid.into(),
                    object_type: gameobjects::object::TypeBitField::new()
                        .with_object(true)
                        .with_item(true)
                        .into(),
                    entry: 789.into(),
                    scale_x: 1.0.into(),
                    _padding: 0.into(),
                })
                .write_full_update_block(&mut mask_blocks, &mut values_blocks);
                (gameobjects::item::ItemFields {
                    owner: Some(*character.object_fields.guid.get()).into(),
                    //owner: None.into(),
                    contained_in: None.into(),
                    creator: None.into(),
                    gift_creator: None.into(),
                    stack_count: 2.into(),
                    expires_in: None.into(),
                    spell_charges: [0.into(); 5],
                    flags: gameobjects::item::ItemFlags::new().into(),
                    enchantments: [gameobjects::item::ItemEnchantment {
                        id: 0,
                        duration: 0,
                        charges: 0,
                    }
                    .into(); 9],
                    property_seed: 0.into(),
                    random_properties_id: 1.into(),
                    item_text_id: 0.into(),
                    durability: 60.into(),
                    max_durability: 60.into(),
                    _padding: 0.into(),
                })
                .write_full_update_block(&mut mask_blocks, &mut values_blocks);

                UpdateData::CreateNewObject {
                    guid: AnyGuid::Item(guid),
                    movement: MovementUpdate {
                        is_self_update: false,
                        position: None,
                        high_guid: Some(guid::Item::get_prefix() as u32),
                        is_update_all: true,
                        full_guid: PossibleUpdate::NoUpdate,
                        transport_time_millis: None,
                    },
                    values: ValuesUpdate {
                        mask_blocks: mask_blocks,
                        values_blocks: values_blocks,
                    },
                }
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

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks: vec![block, test_block],
                },
            };

            if let Err(e) = lock
                .write_all(&response.to_bytes(Some(character.session_key)))
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
            let stream_tx = character.stream_tx.clone();

            self.characters
                .insert(character.object_fields.guid.get().clone(), character);

            tokio::task::spawn(packet_handler(
                rx,
                stream_tx,
                session_key,
                character_id,
                self.player_update_queue.clone(),
                self.game_data_accessor.clone(),
            ));
        }
    }

    async fn send_updates_to_players(&mut self) {
        for character in self.characters.values_mut() {
            let mut lock = character.stream_tx.lock().await;

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

            let response = packets::server::SMSG_UPDATE_OBJECT {
                update_data: UpdateBlocks {
                    has_transport: false,
                    blocks: vec![block],
                },
            };

            if let Err(e) = lock
                .write_all(&response.to_bytes(Some(character.session_key)))
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
}

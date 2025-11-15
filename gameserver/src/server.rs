use std::sync::Arc;

use bit_vec::BitVec;
use chrono::{DateTime, Local, TimeDelta};
use common::guid::AnyGuid;
use concurrent_queue::ConcurrentQueue;
use gameobjects::tracked_field::ClientUpdatable;
use log::error;
use packets::update_data::{
    MovementFlags, MovementInfo, MovementUpdate, PositionUpdate, PossibleUpdate, UpdateBlocks,
    UpdateData, ValuesUpdate,
};
use tokio::io::AsyncWriteExt;

use crate::character::Character;

pub struct Server {
    pub game_time: DateTime<Local>,

    characters: Vec<Character>,

    world_transition_character_queue: Arc<ConcurrentQueue<Character>>,
}

impl Server {
    pub fn new(world_transition_character_queue: Arc<ConcurrentQueue<Character>>) -> Self {
        Self {
            game_time: Local::now(),

            characters: Vec::new(),

            world_transition_character_queue,
        }
    }

    pub async fn update(&mut self, diff: TimeDelta) {
        self.add_queued_characters().await;
    }

    async fn add_queued_characters(&mut self) {
        for mut character in self.world_transition_character_queue.try_iter() {
            let response = packets::server::SMSG_LOGIN_VERIFY_WORLD {
                map: character.map_id,
                position_x: character.position.0,
                position_y: character.position.1,
                position_z: character.position.2,
                orientation: character.orientation,
            };

            if let Err(e) = character
                .stream
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

            if let Err(e) = character
                .stream
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

            if let Err(e) = character
                .stream
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

            if let Err(e) = character
                .stream
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

            if let Err(e) = character
                .stream
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

            if let Err(e) = character
                .stream
                .write_all(&response.to_bytes(Some(character.session_key)))
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
                        run_speed: 70.0,
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
                    blocks: vec![block],
                },
            };

            if let Err(e) = character
                .stream
                .write_all(&response.to_bytes(Some(character.session_key)))
                .await
            {
                error!(
                    "Failed to send SMSG_UPDATE_OBJECT to client (account_id: {}). Error: {:?}",
                    character.account_id, e
                );
                return;
            };

            self.characters.push(character);
        }
    }
}

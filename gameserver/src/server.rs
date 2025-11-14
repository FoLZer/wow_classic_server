use std::sync::Arc;

use chrono::{DateTime, Local, TimeDelta};
use concurrent_queue::ConcurrentQueue;
use log::error;
use tokio::io::AsyncWriteExt;

use crate::character::Character;

pub struct Server {
    pub game_time: DateTime<Local>,

    world_transition_character_queue: Arc<ConcurrentQueue<Character>>,
}

impl Server {
    pub fn new(world_transition_character_queue: Arc<ConcurrentQueue<Character>>) -> Self {
        Self {
            game_time: Local::now(),

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
        }
    }
}

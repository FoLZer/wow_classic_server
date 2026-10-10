mod combat;
mod player_updates;
mod spawning;
mod visibility;

use std::{collections::HashMap, sync::Arc, time::Instant};

use chrono::{DateTime, Local, TimeDelta};
use common::guid::{self, Guid};
use concurrent_queue::ConcurrentQueue;
use log::warn;
use packets::server::ServerPacket;
use tokio::{io::AsyncWriteExt, net::tcp::OwnedReadHalf, sync::mpsc};

use crate::{
    creature_spawner::CreatureSpawner,
    game_data::GameDataAccessor,
    guid_allocator::GuidAllocator,
    objects::{character::Character, creature::Creature},
    packet_handler::PlayerUpdate,
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

        Self::do_initial_spawns(
            &mut creature_spawners,
            &game_data_accessor,
            &mut unit_guid_allocator,
            &mut creatures,
        )
        .await;

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

    async fn broadcast_packet<T: ServerPacket, F: Fn(&Character) -> bool>(
        &self,
        packet: &T,
        except: F,
    ) {
        for character in self.characters.values() {
            if except(character) {
                continue;
            }

            let mut lock = character.stream_tx.lock().await;

            if let Err(e) = lock
                .write_all(&packet.to_bytes(
                    Some(character.session_key),
                    &mut *character.encrypt_data.lock().await,
                ))
                .await
            {
                warn!(
                    "Failed to send {} to client (account_id: {}). Error: {:?}",
                    T::PACKET_NAME,
                    character.account_id,
                    e
                );
                continue;
            };
        }
    }
}

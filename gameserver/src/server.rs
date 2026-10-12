mod combat;
mod despawning;
mod player_updates;
mod spawning;
mod visibility;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Local, TimeDelta};
use common::guid::{self, Guid};
use concurrent_queue::ConcurrentQueue;
use log::{info, warn};
use packets::server::ServerPacket;
use sqlx::{Pool, Sqlite};
use tokio::{io::AsyncWriteExt, net::tcp::OwnedReadHalf, sync::mpsc};

use crate::{
    creature_spawner::CreatureSpawner,
    dirty_tracking_map::DirtyTrackingMap,
    game_data::GameDataAccessor,
    guid_allocator::GuidAllocator,
    objects::{character::Character, creature::Creature, creature_corpse::CreatureCorpse},
    packet_handler::PlayerUpdate,
    sparse_set::SparseSet,
};

pub struct Server {
    pub game_time: DateTime<Local>,

    // TODO: This needs to be either a configuration or dynamically computed
    corpse_despawn_time: Duration,
    guid_quarantine_time: Duration,
    say_message_range: f32,
    yell_message_range: f32,

    creature_spawners: SparseSet<CreatureSpawner>,
    creatures: DirtyTrackingMap<Guid<guid::Unit>, Creature>,
    characters: HashMap<Guid<guid::Player>, Character>,
    creature_corpses: DirtyTrackingMap<Guid<guid::Unit>, CreatureCorpse>,

    unit_guid_allocator: GuidAllocator<guid::Unit>,

    // A queue containing all parsed updates received from players during this tick
    player_update_queue: Arc<ConcurrentQueue<PlayerUpdate>>,
    world_transition_character_queue:
        Arc<ConcurrentQueue<(Box<Character>, OwnedReadHalf, (usize, u8))>>,
    character_transition_to_character_screen_tx:
        mpsc::UnboundedSender<(Character, OwnedReadHalf, (usize, u8))>,
    game_data_accessor: GameDataAccessor,
    db: Pool<Sqlite>,
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
        db: Pool<Sqlite>,
    ) -> Result<Self, sqlx::Error> {
        let mut unit_guid_allocator = GuidAllocator::new();
        let mut creatures = DirtyTrackingMap::default();

        let now = Instant::now();

        info!("Started server world loading.");

        let creature_spawners = game_data_accessor.load_creature_spawners().await?;

        info!(
            "Creature spawners loaded in {} ms. Spawners loaded: {}",
            now.elapsed().as_millis(),
            creature_spawners.len(),
        );

        let mut creature_spawners = SparseSet::from_vec(creature_spawners);

        Self::do_initial_spawns(
            &mut creature_spawners,
            &game_data_accessor,
            &mut unit_guid_allocator,
            &mut creatures,
        )
        .await;

        info!(
            "World loading done in {} ms. Creatures created: {}",
            now.elapsed().as_millis(),
            creatures.len()
        );

        Ok(Self {
            game_time: Local::now(),

            corpse_despawn_time: Duration::from_secs(10),
            guid_quarantine_time: Duration::from_secs(5),
            say_message_range: 500.0,
            yell_message_range: 90000.0,

            creature_spawners,

            creatures,
            characters: HashMap::new(),
            creature_corpses: DirtyTrackingMap::default(),

            unit_guid_allocator,

            player_update_queue: Arc::new(ConcurrentQueue::unbounded()),
            world_transition_character_queue,
            character_transition_to_character_screen_tx,
            game_data_accessor,
            db,
        })
    }

    pub async fn update(&mut self, diff: TimeDelta) {
        let now = Instant::now();
        self.add_queued_characters().await;
        self.process_player_updates().await;

        self.despawn_corpses(now).await;
        self.release_quarantined_guids(now);
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

    async fn broadcast_packet_in_range<T: ServerPacket, F: Fn(&Character) -> bool>(
        &self,
        packet: &T,
        source_position: (f32, f32, f32),
        range: f32,
        except: F,
    ) {
        for character in self.characters.values() {
            if character.distance_to_squared(source_position) > range || except(character) {
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

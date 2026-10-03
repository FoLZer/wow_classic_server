use std::time::{Duration, Instant};

pub enum CreatureSpawner {
    Static(StaticCreatureSpawner),
}

impl CreatureSpawner {
    pub fn notify_creature_died(&mut self) {
        match self {
            CreatureSpawner::Static(spawner) => spawner.notify_creature_died(),
        }
    }

    // Skips the respawn time check, used to create creatures immediately on server startup instead of waiting for respawn
    // (spawnInfo, keepActive)
    pub fn force_first_spawn(&self) -> (Vec<CreatureSpawnInfo>, bool) {
        match self {
            CreatureSpawner::Static(spawner) => spawner.force_first_spawn(),
        }
    }

    // (spawnInfo, keepActive)
    pub fn get_creatures_to_spawn(&self) -> (Vec<CreatureSpawnInfo>, bool) {
        match self {
            CreatureSpawner::Static(spawner) => spawner.get_creatures_to_spawn(),
        }
    }
}

pub struct StaticCreatureSpawner {
    position: (f32, f32, f32),
    orientation: f32,

    spawn_creature_id: u32,

    respawn_time: Duration,
    died_at: Instant,
}

impl StaticCreatureSpawner {
    pub fn new(
        position: (f32, f32, f32),
        orientation: f32,
        spawn_creature_id: u32,
        respawn_time: Duration,
    ) -> Self {
        Self {
            position,
            orientation,
            spawn_creature_id,
            respawn_time,
            died_at: Instant::now(),
        }
    }

    pub fn notify_creature_died(&mut self) {
        self.died_at = Instant::now();
    }

    pub fn force_first_spawn(&self) -> (Vec<CreatureSpawnInfo>, bool) {
        (
            vec![CreatureSpawnInfo {
                position: self.position,
                orientation: self.orientation,
                spawn_creature_id: self.spawn_creature_id,
            }],
            false,
        )
    }

    pub fn get_creatures_to_spawn(&self) -> (Vec<CreatureSpawnInfo>, bool) {
        if self.died_at.elapsed() > self.respawn_time {
            (
                vec![CreatureSpawnInfo {
                    position: self.position,
                    orientation: self.orientation,
                    spawn_creature_id: self.spawn_creature_id,
                }],
                false,
            )
        } else {
            (Vec::new(), true)
        }
    }
}

pub struct CreatureSpawnInfo {
    pub position: (f32, f32, f32),
    pub orientation: f32,
    pub spawn_creature_id: u32,
}

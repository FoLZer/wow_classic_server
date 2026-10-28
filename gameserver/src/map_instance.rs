use crate::{creature_spawner::CreatureSpawner, sparse_set::SparseSet};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InstanceId(u32);

impl InstanceId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn get(&self) -> u32 {
        self.0
    }
}

pub struct MapInstance {
    pub map_id: u32,

    pub creature_spawners: SparseSet<CreatureSpawner>,
}

use std::time::Instant;

use common::guid::{self, LivingGuid};
use gameobjects::{object::ObjectFields, unit::UnitFields};

pub struct Creature {
    pub position: (f32, f32, f32),
    pub orientation: f32,

    pub object_fields: ObjectFields<guid::Unit>,
    pub unit_fields: UnitFields,

    pub spawner_index: Option<usize>,
    pub melee_state: Option<MeleeState>,
}

pub struct MeleeState {
    // Can be none to preserve last_main_hand and last_off_hand
    // The idea is to clear the entire melee_state once both of these cooldowns pass and victim is None
    pub victim: Option<LivingGuid>,
    pub last_main_hand: Option<Instant>,
    pub last_off_hand: Option<Instant>,
}

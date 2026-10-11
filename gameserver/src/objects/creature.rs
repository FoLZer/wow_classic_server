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
    pub vendor_menu: Option<Vec<VendorItem>>,
}

impl Creature {
    pub fn is_alive(&self) -> bool {
        *self.unit_fields.health.get() != 0
    }
}

pub struct MeleeState {
    // Can be none to preserve last_main_hand and last_off_hand
    // The idea is to clear the entire melee_state once both of these cooldowns pass and victim is None
    pub victim: Option<LivingGuid>,
    pub last_main_hand: Option<Instant>,
    pub last_off_hand: Option<Instant>,
}

pub struct VendorItem {
    pub item_id: u32,
    pub display_id: u32,
    pub num_available: Option<u32>,
    pub price: u32,
    pub durability: u32,
    pub purchase_amount: u32,
}

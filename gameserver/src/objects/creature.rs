use std::time::Instant;

use common::guid::{self, LivingGuid};
use gamedata::{creature_prototype::VendorItem, gossip::GossipMenuTree};
use gameobjects::{object::ObjectFields, tracked_field::ClientUpdatable, unit::UnitFields};

use crate::{dirty_tracking_map::DirtyTrackable, map_instance::InstanceId};

pub struct Creature {
    pub instance_id: InstanceId,
    pub position: (f32, f32, f32),
    pub orientation: f32,

    pub object_fields: ObjectFields<guid::Unit>,
    pub unit_fields: UnitFields,

    pub spawner_index: Option<usize>,
    pub melee_state: Option<MeleeState>,
    pub vendor_menu: Option<Vec<VendorItem>>,
    pub gossip_menu: Option<GossipMenuTree>,
}

impl Creature {
    pub fn is_alive(&self) -> bool {
        *self.unit_fields.health.get() != 0
    }
}

impl DirtyTrackable for Creature {
    fn is_dirty(&self) -> bool {
        self.object_fields.has_updates() || self.unit_fields.has_updates()
    }
}

pub struct MeleeState {
    // Can be none to preserve last_main_hand and last_off_hand
    // The idea is to clear the entire melee_state once both of these cooldowns pass and victim is None
    pub victim: Option<LivingGuid>,
    pub last_main_hand: Option<Instant>,
    pub last_off_hand: Option<Instant>,
    pub last_swing_error: Option<Instant>,
}

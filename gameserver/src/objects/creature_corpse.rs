use std::time::Instant;

use common::guid;
use gameobjects::{object::ObjectFields, tracked_field::ClientUpdatable, unit::UnitFields};

use crate::{dirty_tracking_map::DirtyTrackable, map_instance::InstanceId};

pub struct CreatureCorpse {
    pub instance_id: InstanceId,
    pub position: (f32, f32, f32),
    pub orientation: f32,

    pub object_fields: ObjectFields<guid::Unit>,
    pub unit_fields: UnitFields,

    pub died_at: Instant,
}

impl DirtyTrackable for CreatureCorpse {
    fn is_dirty(&self) -> bool {
        self.object_fields.has_updates() || self.unit_fields.has_updates()
    }
}

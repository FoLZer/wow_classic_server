use common::guid;
use gameobjects::{object::ObjectFields, unit::UnitFields};

pub struct Creature {
    pub position: (f32, f32, f32),
    pub orientation: f32,

    pub object_fields: ObjectFields<guid::Unit>,
    pub unit_fields: UnitFields,
}

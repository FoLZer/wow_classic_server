use bitfield_struct::bitfield;
use common::guid::{self, Guid};

pub struct ObjectFields<GuidType: guid::GuidType> {
    pub guid: Guid<GuidType>,
    pub object_type: TypeBitField,
    pub entry: u32,
    pub scale_x: f32,
    pub _padding: u32,
}

#[bitfield(u8)]
pub struct TypeBitField {
    pub object: bool,
    pub item: bool,
    pub container: bool,
    pub unit: bool,
    pub player: bool,
    pub game_object: bool,
    pub dynamic_object: bool,
    pub corpse: bool,
}

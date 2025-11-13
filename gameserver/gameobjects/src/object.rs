use bitfield_struct::bitfield;
use common::guid::{self, Guid};

use crate::tracked_field::{ClientUpdatable, TrackedField, UpdateWritable};

pub struct ObjectFields<GuidType: guid::GuidType> {
    pub guid: TrackedField<Guid<GuidType>>,
    pub object_type: TrackedField<TypeBitField>,
    pub entry: TrackedField<u32>,
    pub scale_x: TrackedField<f32>,
    pub _padding: TrackedField<u32>,
}

impl<T: guid::GuidType> ClientUpdatable for ObjectFields<T> {
    fn write_update_block(&self, mask_bits: &mut bit_vec::BitVec<u32>, values_buf: &mut Vec<u32>) {
        self.guid.write(mask_bits, values_buf);
        self.object_type.write(mask_bits, values_buf);
        self.entry.write(mask_bits, values_buf);
        self.scale_x.write(mask_bits, values_buf);
        self._padding.write(mask_bits, values_buf);
    }

    fn write_full_update_block(
        &self,
        mask_bits: &mut bit_vec::BitVec<u32>,
        values_buf: &mut Vec<u32>,
    ) {
        self.guid.write_forced(mask_bits, values_buf);
        self.object_type.write_forced(mask_bits, values_buf);
        self.entry.write_forced(mask_bits, values_buf);
        self.scale_x.write_forced(mask_bits, values_buf);
        self._padding.write_forced(mask_bits, values_buf);
    }
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

impl UpdateWritable for TypeBitField {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0 as u32;
    }
}

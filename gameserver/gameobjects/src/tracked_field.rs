use bit_vec::BitVec;
use common::guid::{Guid, GuidType};

pub trait UpdateWritable {
    fn get_mask_bits_count() -> usize {
        1
    }

    fn get_update_blocks_count() -> usize {
        1
    }

    // As an argument, provides a mutable slice containing N u32's as was specified in get_update_blocks_count
    // The bytes in blocks will be initialized to 0's
    fn write(&self, blocks: &mut [u32]);
}

//TODO: At the moment mask_bits will be populated with leading zeros which shouldn't really happen
//This is solvable by statically calculating indexes for each field but right now I'm not sure how to do this in a macro
pub trait ClientUpdatable {
    fn write_update_block(&self, mask_bits: &mut BitVec<u32>, values_buf: &mut Vec<u32>);

    fn write_full_update_block(&self, mask_bits: &mut BitVec<u32>, values_buf: &mut Vec<u32>);
}

#[derive(Clone, Copy)]
pub struct TrackedField<T> {
    is_updated: bool,
    field: T,
}

impl<T> TrackedField<T> {
    pub fn get(&self) -> &T {
        &self.field
    }

    pub fn is_updated(&self) -> bool {
        self.is_updated
    }

    // In practice, this method should never be called unless there's an error somewhere
    pub fn force_update(&mut self) {
        self.is_updated = true;
    }
}

impl<T: UpdateWritable> TrackedWriteTrait for TrackedField<T> {
    fn write(&self, mask_bits: &mut BitVec<u32>, values_buf: &mut Vec<u32>) {
        if self.is_updated {
            self.write_forced(mask_bits, values_buf);
        } else {
            for _ in 0..T::get_mask_bits_count() {
                mask_bits.push(false);
            }
        }
    }

    fn write_forced(&self, mask_bits: &mut BitVec<u32>, values_buf: &mut Vec<u32>) {
        for _ in 0..T::get_mask_bits_count() {
            mask_bits.push(true);
        }

        let count = T::get_update_blocks_count();

        values_buf.resize(values_buf.len() + count, 0);

        let len = values_buf.len();
        let blocks = &mut values_buf[len - count..];
        self.field.write(blocks);
    }
}

impl<T: UpdateWritable, const N: usize> TrackedWriteTrait for [TrackedField<T>; N] {
    fn write(&self, mask_bits: &mut BitVec<u32>, values_buf: &mut Vec<u32>) {
        for v in self {
            v.write(mask_bits, values_buf);
        }
    }

    fn write_forced(&self, mask_bits: &mut BitVec<u32>, values_buf: &mut Vec<u32>) {
        for v in self {
            v.write_forced(mask_bits, values_buf);
        }
    }
}

pub(crate) trait TrackedWriteTrait {
    fn write(&self, mask_bits: &mut BitVec<u32>, values_buf: &mut Vec<u32>);
    fn write_forced(&self, mask_bits: &mut BitVec<u32>, values_buf: &mut Vec<u32>);
}

impl<T> From<T> for TrackedField<T> {
    fn from(value: T) -> Self {
        Self {
            field: value,
            is_updated: false,
        }
    }
}

impl UpdateWritable for u32 {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = *self;
    }
}

impl UpdateWritable for f32 {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.to_bits();
    }
}

impl<T: GuidType> UpdateWritable for Guid<T> {
    fn get_update_blocks_count() -> usize {
        2
    }

    fn write(&self, blocks: &mut [u32]) {
        let b = self.get().get();
        let b1 = (b & 0x0000_0000_FFFF_FFFFu64) as u32;
        let b2 = (b >> 32) as u32;
        blocks[0] = b1;
        blocks[1] = b2;
    }
}

impl<T: GuidType> UpdateWritable for Option<Guid<T>> {
    fn get_update_blocks_count() -> usize {
        2
    }

    fn write(&self, blocks: &mut [u32]) {
        if let Some(v) = self {
            v.write(blocks);
        }
    }
}

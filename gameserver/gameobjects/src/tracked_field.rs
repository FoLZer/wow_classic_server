use std::{
    hash::{DefaultHasher, Hash, Hasher},
    marker::PhantomData,
    num::NonZeroU32,
    ops::{Deref, DerefMut},
};

use bit_vec::BitVec;
use common::guid::{AnyGuid, Guid, GuidType};

pub trait UpdateWritable {
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
    fn clear_update_flags(&mut self);

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

    pub fn clear_update_flag(&mut self) {
        self.is_updated = false;
    }
}

impl<T: PartialEq> PartialEq<T> for TrackedField<T> {
    fn eq(&self, other: &T) -> bool {
        &self.field == other
    }
}

pub struct CopyCheck<T: Eq> {
    initial_value: T,
}

pub struct HashCheck<T: Hash> {
    initial_hash: u64,
    _phantom: PhantomData<T>,
}

pub trait Check<T> {
    fn check_changed(&self, value: &T) -> bool;
}

impl<T: Eq> Check<T> for CopyCheck<T> {
    fn check_changed(&self, value: &T) -> bool {
        self.initial_value.ne(value)
    }
}

impl<T: Hash> Check<T> for HashCheck<T> {
    fn check_changed(&self, value: &T) -> bool {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();

        self.initial_hash != hash
    }
}

pub struct TrackedFieldGuard<'a, T, C: Check<T>> {
    field_ref: &'a mut TrackedField<T>,
    update_check_data: C,
}

impl<'a, T, C: Check<T>> Deref for TrackedFieldGuard<'a, T, C> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.field_ref.field
    }
}

impl<'a, T, C: Check<T>> DerefMut for TrackedFieldGuard<'a, T, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.field_ref.field
    }
}

impl<'a, T, C: Check<T>> Drop for TrackedFieldGuard<'a, T, C> {
    fn drop(&mut self) {
        if self.update_check_data.check_changed(&self.field_ref.field) {
            self.field_ref.is_updated = true;
        }
    }
}

impl<'a, T: Eq + Copy> TrackedField<T> {
    pub fn get_mut_using_copy(&'a mut self) -> TrackedFieldGuard<'a, T, CopyCheck<T>> {
        TrackedFieldGuard {
            update_check_data: CopyCheck {
                initial_value: self.field,
            },
            field_ref: self,
        }
    }
}

impl<'a, T: Hash> TrackedField<T> {
    pub fn get_mut_using_hash(&'a mut self) -> TrackedFieldGuard<'a, T, HashCheck<T>> {
        let mut hasher = DefaultHasher::new();
        self.field.hash(&mut hasher);
        let hash = hasher.finish();

        TrackedFieldGuard {
            update_check_data: HashCheck {
                initial_hash: hash,
                _phantom: PhantomData,
            },
            field_ref: self,
        }
    }
}

impl<T: UpdateWritable> TrackedWriteTrait for TrackedField<T> {
    fn write(&self, mask_bits: &mut BitVec<u32>, values_buf: &mut Vec<u32>) {
        if self.is_updated {
            self.write_forced(mask_bits, values_buf);
        } else {
            for _ in 0..T::get_update_blocks_count() {
                mask_bits.push(false);
            }
        }
    }

    fn write_forced(&self, mask_bits: &mut BitVec<u32>, values_buf: &mut Vec<u32>) {
        let count = T::get_update_blocks_count();

        for _ in 0..count {
            mask_bits.push(true);
        }

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

impl UpdateWritable for AnyGuid {
    fn get_update_blocks_count() -> usize {
        2
    }

    fn write(&self, blocks: &mut [u32]) {
        match self {
            AnyGuid::Item(guid) => guid.write(blocks),
            AnyGuid::Container(guid) => guid.write(blocks),
            AnyGuid::Unit(guid) => guid.write(blocks),
            AnyGuid::Player(guid) => guid.write(blocks),
            AnyGuid::GameObject(guid) => guid.write(blocks),
            AnyGuid::DynamicObject(guid) => guid.write(blocks),
            AnyGuid::Corpse(guid) => guid.write(blocks),
        }
    }
}

impl UpdateWritable for Option<AnyGuid> {
    fn get_update_blocks_count() -> usize {
        2
    }

    fn write(&self, blocks: &mut [u32]) {
        if let Some(v) = self {
            v.write(blocks);
        }
    }
}

impl UpdateWritable for Option<NonZeroU32> {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.map_or(0, |v| v.get());
    }
}

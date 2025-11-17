use std::num::NonZeroU32;

use bitfield_struct::bitfield;
use common::guid::{self, AnyGuid, Guid};
use macros::tracked;

use crate::tracked_field::{TrackedWriteTrait, UpdateWritable};

#[tracked]
pub struct ItemFields {
    pub owner: Option<Guid<guid::Player>>,
    pub contained_in: Option<AnyGuid>,
    pub creator: Option<Guid<guid::Player>>,
    pub gift_creator: Option<Guid<guid::Player>>,
    pub stack_count: u32,
    pub expires_in: Option<NonZeroU32>,
    pub spell_charges: [u32; 5],
    pub flags: ItemFlags,
    pub enchantments: [ItemEnchantment; 9],
    pub property_seed: u32,
    pub random_properties_id: u32,
    pub item_text_id: u32,
    pub durability: u32,
    pub max_durability: u32,
    pub _padding: u32,
}

#[bitfield(u32)]
pub struct ItemFlags {
    pub is_binded: bool,
    _unknown: bool,
    // safes, crates, etc.
    pub is_unlocked: bool,
    pub is_wrapped: bool,
    _unknown: bool,
    _unknown: bool,
    _unknown: bool,
    _unknown: bool,
    _unknown: bool,
    pub is_readable: bool,
    #[bits(22)]
    _unknown: u32,
}

impl UpdateWritable for ItemFlags {
    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.0;
    }
}

#[derive(Clone, Copy)]
pub struct ItemEnchantment {
    pub id: u32,
    pub duration: u32,
    pub charges: u32,
}

impl UpdateWritable for ItemEnchantment {
    fn get_update_blocks_count() -> usize {
        3
    }

    fn write(&self, blocks: &mut [u32]) {
        blocks[0] = self.id;
        blocks[1] = self.duration;
        blocks[2] = self.charges;
    }
}

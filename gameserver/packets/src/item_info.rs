use bitfield_struct::bitfield;
use byteorder::{ByteOrder, LittleEndian};

use crate::server::OrderedWrite;

#[derive(Clone, Copy, Debug)]
pub struct ItemStat {
    pub ty: u32,
    pub value: u32,
}

impl<T: ByteOrder> OrderedWrite<T> for ItemStat {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <u32 as OrderedWrite<LittleEndian>>::write(&self.ty, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.value, writer)?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ItemDamage {
    pub min: u32,
    pub max: u32,
    pub ty: u32,
}

impl<T: ByteOrder> OrderedWrite<T> for ItemDamage {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <u32 as OrderedWrite<LittleEndian>>::write(&self.min, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.max, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.ty, writer)?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ItemSpell {
    pub id: u32,
    pub trigger: u32,
    pub charges: u32,
    pub cooldown: u32,
    pub category: u32,
    pub category_cooldown: u32,
}

impl<T: ByteOrder> OrderedWrite<T> for Option<ItemSpell> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        if let Some(v) = self {
            <u32 as OrderedWrite<LittleEndian>>::write(&v.id, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(&v.trigger, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(&v.charges, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(&v.cooldown, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(&v.category, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(&v.category_cooldown, writer)?;
        } else {
            <u32 as OrderedWrite<LittleEndian>>::write(&0, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(&0, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(&0, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(&u32::MAX, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(&0, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(&u32::MAX, writer)?;
        }

        Ok(())
    }
}

#[bitfield(u32)]
pub struct ItemFlags {
    _reserved: bool,
    pub conjured: bool,
    pub lootable: bool,
    _unused: bool, //ITEM_FLAG_WRAPPED
    pub deprecated: bool,
    pub indestructible: bool,
    pub usable: bool,
    pub no_equip_cooldown: bool,
    _reserved: bool,
    pub wrapper: bool,
    pub stackable: bool,
    pub party_loot: bool,
    _reserved: bool,
    pub guild_charter: bool,
    pub letter: bool,
    pub pvp_reward: bool,
    _unknown: bool,
    _unknown: bool,

    #[bits(14)]
    _unused: u16,
}

impl<T: ByteOrder> OrderedWrite<T> for ItemFlags {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <u32 as OrderedWrite<LittleEndian>>::write(&self.0, writer)?;

        Ok(())
    }
}

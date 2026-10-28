use std::num::NonZeroU32;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};

use crate::server::OrderedWrite;

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum TrainerType {
    Class = 0,
    Talent = 1,
    Tradeskill = 2,
    Pets = 3,
}

impl<T: ByteOrder> OrderedWrite<T> for TrainerType {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u32::<T>(*self as u32)
    }

    fn serialized_size(&self) -> usize {
        size_of::<u32>()
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum TrainerSpellState {
    Available = 0,
    Unavailable = 1,
    AlreadyKnown = 2,
}

pub struct TrainerSpell {
    pub spell_id: u32,
    pub state: TrainerSpellState,
    pub money_cost: u32,
    pub talent_point_cost: u32,
    pub profession_point_cost: u32,
    pub required_level: u8,
    pub required_skill: u32,
    pub required_skill_value: u32,
    pub required_spells: [Option<NonZeroU32>; 3],
}

impl<T: ByteOrder> OrderedWrite<T> for TrainerSpell {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <u32 as OrderedWrite<LittleEndian>>::write(&self.spell_id, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&(self.state as u8), writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.money_cost, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.talent_point_cost, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.profession_point_cost, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.required_level, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.required_skill, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.required_skill_value, writer)?;
        <[Option<NonZeroU32>; 3] as OrderedWrite<LittleEndian>>::write(
            &self.required_spells,
            writer,
        )?;

        Ok(())
    }
}

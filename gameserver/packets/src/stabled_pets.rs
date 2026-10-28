use std::ffi::CString;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};

use crate::server::OrderedWrite;

pub struct StabledPets {
    /// Number of stable slots the player has bought
    pub stable_slots: u8,
    pub pets: Vec<StabledPet>,
}

impl<T: ByteOrder> OrderedWrite<T> for StabledPets {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u8(self.pets.len().min(u8::MAX as usize) as u8)?;
        writer.write_u8(self.stable_slots)?;
        for pet in &self.pets {
            <StabledPet as OrderedWrite<LittleEndian>>::write(pet, writer)?;
        }

        Ok(())
    }
}

pub struct StabledPet {
    pub pet_number: u32,
    pub entry: u32,
    pub level: u32,
    pub name: CString,
    pub loyalty: u32,
    pub slot: StabledPetSlot,
}

impl<T: ByteOrder> OrderedWrite<T> for StabledPet {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <u32 as OrderedWrite<LittleEndian>>::write(&self.pet_number, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.entry, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.level, writer)?;
        <CString as OrderedWrite<LittleEndian>>::write(&self.name, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.loyalty, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&(self.slot as u8), writer)?;

        Ok(())
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum StabledPetSlot {
    Active = 1,
    Stabled = 2,
}

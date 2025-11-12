use std::{ffi::CString, str::FromStr};

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};

use crate::{guid::{self, Guid, GuidType}, server::OrderedWrite};

#[derive(Clone)]
pub struct CharacterInfo {
    pub guid: Guid<guid::Player>,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub skin: u8,
    pub face: u8,
    pub hairstyle: u8,
    pub haircolor: u8,
    pub facialhair: u8,
    pub level: u8,
    pub area: u32,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub guild_id: u32,
    pub flags: u32,
    pub first_login: bool,
    pub pet_display_id: u32,
    pub pet_level: u32,
    pub pet_family: u32,
    pub equipment: Equipment,
    pub first_bag_display_id: u32,
    pub first_bag_inventory_type: u8,
}

#[derive(Clone)]
pub struct Equipment {
    pub head: GearInfo,
    pub neck: GearInfo,
    pub shoulders: GearInfo,
    pub body: GearInfo,
    pub chest: GearInfo,
    pub waist: GearInfo,
    pub legs: GearInfo,
    pub feet: GearInfo,
    pub wrists: GearInfo,
    pub hands: GearInfo,
    pub finger1: GearInfo,
    pub finger2: GearInfo,
    pub trinket1: GearInfo,
    pub trinket2: GearInfo,
    pub back: GearInfo,
    pub mainhand: GearInfo,
    pub offhand: GearInfo,
    pub ranged: GearInfo,
    pub tabard: GearInfo,
}

#[derive(Clone)]
pub struct GearInfo {
    pub display_id: u32,
    pub inventory_type: u8,
}

impl<T: ByteOrder> OrderedWrite<T> for CharacterInfo {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <Guid<guid::Player> as OrderedWrite<LittleEndian>>::write(&self.guid, writer)?;
        <CString as OrderedWrite<LittleEndian>>::write(&CString::from_str(&self.name)?, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.race, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.class, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.gender, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.skin, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.face, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.hairstyle, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.haircolor, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.facialhair, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.level, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.area, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.map, writer)?;
        <f32 as OrderedWrite<LittleEndian>>::write(&self.position_x, writer)?;
        <f32 as OrderedWrite<LittleEndian>>::write(&self.position_y, writer)?;
        <f32 as OrderedWrite<LittleEndian>>::write(&self.position_z, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.guild_id, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.flags, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&(self.first_login as u8), writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.pet_display_id, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.pet_level, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.pet_family, writer)?;
        <Equipment as OrderedWrite<LittleEndian>>::write(&self.equipment, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.first_bag_display_id, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.first_bag_inventory_type, writer)?;

        Ok(())
    }
}

impl<T: ByteOrder, Type: GuidType> OrderedWrite<T> for Guid<Type> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u64::<T>(self.get().get())
    }
}

impl<T: ByteOrder> OrderedWrite<T> for Equipment {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.head, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.neck, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.shoulders, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.body, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.chest, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.waist, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.legs, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.feet, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.wrists, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.hands, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.finger1, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.finger2, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.trinket1, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.trinket2, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.back, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.mainhand, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.offhand, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.ranged, writer)?;
        <GearInfo as OrderedWrite<LittleEndian>>::write(&self.tabard, writer)?;

        Ok(())
    }
}

impl<T: ByteOrder> OrderedWrite<T> for GearInfo {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <u32 as OrderedWrite<LittleEndian>>::write(&self.display_id, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.inventory_type, writer)?;

        Ok(())
    }
}

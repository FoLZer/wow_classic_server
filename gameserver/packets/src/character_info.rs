use std::ffi::CString;

use bitfield_struct::bitfield;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use common::guid::{self, Guid, GuidType};
use gamedata::equipment_slot::{VisibleEquipment, VisibleItem};

use crate::{bounded::Bounded, server::OrderedWrite};

pub type CharacterName = Bounded<CString, 48>;
pub type CharacterList = Bounded<Vec<CharacterInfo>, 10>;

#[derive(Clone)]
pub struct CharacterInfo {
    pub guid: Guid<guid::Player>,
    pub name: CharacterName,
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
    pub flags: CharacterFlags,
    pub first_login: bool,
    pub pet_display_id: u32,
    pub pet_level: u32,
    pub pet_family: u32,
    pub equipment: VisibleEquipment,
    pub first_bag_display_id: u32,
    pub first_bag_inventory_type: u8,
}

#[bitfield(u32)]
pub struct CharacterFlags {
    #[bits(2)]
    _unkn: u8,
    pub locked_for_transfer: bool,
    #[bits(7)]
    _unkn: u8,
    pub hide_helm: bool,
    pub hide_cloak: bool,
    _unkn: bool,
    pub ghost: bool,
    pub locked_awaiting_rename: bool,
    #[bits(17)]
    _unkn: u32,
}

impl<T: ByteOrder> OrderedWrite<T> for CharacterInfo {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <Guid<guid::Player> as OrderedWrite<LittleEndian>>::write(&self.guid, writer)?;
        <CharacterName as OrderedWrite<LittleEndian>>::write(&self.name, writer)?;
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
        <u32 as OrderedWrite<LittleEndian>>::write(&self.flags.0, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&(self.first_login as u8), writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.pet_display_id, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.pet_level, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.pet_family, writer)?;
        <VisibleEquipment as OrderedWrite<LittleEndian>>::write(&self.equipment, writer)?;
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

impl<T: ByteOrder> OrderedWrite<T> for VisibleEquipment {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        let slots = [
            &self.head,
            &self.neck,
            &self.shoulders,
            &self.body,
            &self.chest,
            &self.waist,
            &self.legs,
            &self.feet,
            &self.wrists,
            &self.hands,
            &self.finger1,
            &self.finger2,
            &self.trinket1,
            &self.trinket2,
            &self.back,
            &self.mainhand,
            &self.offhand,
            &self.ranged,
            &self.tabard,
        ];
        for slot in slots {
            if let Some(v) = slot {
                <VisibleItem as OrderedWrite<LittleEndian>>::write(v, writer)?;
            } else {
                <u32 as OrderedWrite<LittleEndian>>::write(&0, writer)?;
                <u8 as OrderedWrite<LittleEndian>>::write(&0, writer)?;
            }
        }

        Ok(())
    }
}

impl<T: ByteOrder> OrderedWrite<T> for VisibleItem {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <u32 as OrderedWrite<LittleEndian>>::write(&self.display_id.get(), writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.inventory_type, writer)?;

        Ok(())
    }
}

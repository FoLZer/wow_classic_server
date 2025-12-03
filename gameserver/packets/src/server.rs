#![allow(non_camel_case_types)]

use std::{
    ffi::CString,
    io::{ErrorKind, Write},
};

use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use macros::create_server_packets;

use crate::{
    account_result::AccountResult,
    character_info::CharacterInfo,
    inventory_change_result::InventoryChangeResult,
    item_info::{ItemDamage, ItemFlags, ItemSpell, ItemStat},
    update_data::UpdateBlocks,
};

create_server_packets!(
SMSG_CHAR_CREATE 0x03A {
    result: AccountResult: LittleEndian,
},
SMSG_CHAR_DELETE 0x03C {
    result: AccountResult: LittleEndian,
},
SMSG_CHAR_ENUM 0x03B {
    characters: Vec<CharacterInfo>: LittleEndian,
},
SMSG_CHAR_LOGIN_FAILED 0x041 {
    result: AccountResult: LittleEndian,
},
SMSG_LOGIN_SETTIMESPEED 0x042 {
    game_time: DateTime<Local>: LittleEndian,
    game_speed: f32: LittleEndian,
},
SMSG_ITEM_QUERY_SINGLE_RESPONSE 0x058 {
    item_id: u32: LittleEndian,
    class: u32: LittleEndian,
    sub_class: u32: LittleEndian,
    name_1: CString: LittleEndian,
    name_2: CString: LittleEndian,
    name_3: CString: LittleEndian,
    name_4: CString: LittleEndian,
    display_info_id: u32: LittleEndian,
    quality: u32: LittleEndian,
    flags: ItemFlags: LittleEndian,
    buy_price: u32: LittleEndian,
    sell_price: u32: LittleEndian,
    inventory_type: u32: LittleEndian, //TODO
    allowable_class: u32: LittleEndian,
    allowable_race: u32: LittleEndian,
    item_level: u32: LittleEndian,
    required_level: u32: LittleEndian,
    required_skill: u32: LittleEndian,
    required_skill_rank: u32: LittleEndian,
    required_spell: u32: LittleEndian,
    required_honor_rank: u32: LittleEndian,
    required_city_rank: u32: LittleEndian,
    required_reputation_faction: u32: LittleEndian,
    required_reputation_rank: u32: LittleEndian,
    max_count: u32: LittleEndian,
    stackable: u32: LittleEndian,
    container_slots: u32: LittleEndian,
    item_stats: [ItemStat; 10]: LittleEndian,
    damage: [ItemDamage; 5]: LittleEndian,

    armor: u32: LittleEndian,
    holy_resistance: u32: LittleEndian,
    fire_resistance: u32: LittleEndian,
    nature_resistance: u32: LittleEndian,
    frost_resistance: u32: LittleEndian,
    shadow_resistance: u32: LittleEndian,
    arcane_resistance: u32: LittleEndian,

    delay: u32: LittleEndian,
    ammo_type: u32: LittleEndian,
    ranged_mod_range: f32: LittleEndian,

    spells: [Option<ItemSpell>; 5]: LittleEndian,

    bonding: u32: LittleEndian,
    description: CString: LittleEndian,
    page_text: u32: LittleEndian,
    language_id: u32: LittleEndian,
    page_material: u32: LittleEndian,
    start_quest: u32: LittleEndian,
    lock_id: u32: LittleEndian,
    material: u32: LittleEndian,
    sheath: u32: LittleEndian,
    random_property: u32: LittleEndian,
    block: u32: LittleEndian,
    item_set: u32: LittleEndian,
    max_durability: u32: LittleEndian,
    area: u32: LittleEndian,
    map: u32: LittleEndian,
    bag_family: u32: LittleEndian,
},
SMSG_UPDATE_OBJECT 0x0A9 {
    update_data: UpdateBlocks: LittleEndian,
},
SMSG_TUTORIAL_FLAGS 0x0FD {
    tutorial_data0: u32: LittleEndian,
    tutorial_data1: u32: LittleEndian,
    tutorial_data2: u32: LittleEndian,
    tutorial_data3: u32: LittleEndian,
    tutorial_data4: u32: LittleEndian,
    tutorial_data5: u32: LittleEndian,
    tutorial_data6: u32: LittleEndian,
    tutorial_data7: u32: LittleEndian,
},
SMSG_INVENTORY_CHANGE_FAILURE 0x112 {
    result: InventoryChangeResult: LittleEndian
},
SMSG_BINDPOINTUPDATE 0x155 {
    homebind_x: f32: LittleEndian,
    homebind_y: f32: LittleEndian,
    homebind_z: f32: LittleEndian,
    homebind_map_id: u32: LittleEndian,
    homebind_area_id: u32: LittleEndian,
},
SMSG_QUERY_TIME_RESPONSE 0x1CF {
    time: DateTime<Local>: LittleEndian
},
SMSG_PONG 0x1DD {
    sequence_id: u32: LittleEndian,
},
SMSG_AUTH_CHALLENGE 0x1EC {
    server_seed: u32: LittleEndian,
},
SMSG_AUTH_RESPONSE 0x1EE {
    result: AccountResult: LittleEndian,
},
SMSG_ACCOUNT_DATA_TIMES 0x209 {
    unkn: [u32; 32]: LittleEndian
},
SMSG_SET_REST_START 0x21E {
    unkn: u32: LittleEndian
},
SMSG_LOGIN_VERIFY_WORLD 0x236 {
    map: u32: LittleEndian,
    position_x: f32: LittleEndian,
    position_y: f32: LittleEndian,
    position_z: f32: LittleEndian,
    orientation: f32: LittleEndian,
},
);

pub trait OrderedWrite<T: ByteOrder> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()>
    where
        Self: Sized;
}

impl<T: ByteOrder> OrderedWrite<T> for u8 {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u8(*self)
    }
}

impl<T: ByteOrder> OrderedWrite<T> for u32 {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u32::<T>(*self)
    }
}

impl<T: ByteOrder> OrderedWrite<T> for f32 {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_f32::<T>(*self)
    }
}

impl<B: ByteOrder, T: OrderedWrite<B>, const N: usize> OrderedWrite<B> for [T; N] {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        for v in self {
            v.write(writer)?;
        }

        Ok(())
    }
}

impl<T: ByteOrder> OrderedWrite<T> for CString {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_all(self.as_bytes_with_nul())
    }
}

impl<B: ByteOrder, T: OrderedWrite<B>> OrderedWrite<B> for Vec<T> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        if self.len() > u8::MAX as usize {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "Vec must not have more than 255 entries to be sent to the client",
            ));
        }

        writer.write_u8(self.len() as u8)?;
        for v in self {
            v.write(writer)?;
        }
        Ok(())
    }
}

impl<T: ByteOrder, Tz: TimeZone> OrderedWrite<T> for DateTime<Tz> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        let year = self.year() as u32 - 2000;
        let v = year << 24
            | self.month() << 20
            | (self.day() - 1) << 14
            | self.weekday().num_days_from_sunday()
            | self.hour() << 6
            | self.minute();

        writer.write_u32::<T>(v)
    }
}

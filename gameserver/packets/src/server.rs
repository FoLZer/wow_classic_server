#![allow(non_camel_case_types)]

use std::{
    ffi::CString,
    io::{ErrorKind, Write},
    sync::Mutex,
};

use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};
use lazy_static::lazy_static;
use macros::create_server_packets;

use crate::{
    account_result::AccountResult, character_info::CharacterInfo, update_data::UpdateBlocks,
};

lazy_static! {
    static ref ENCRYPT_DATA: Mutex<(usize, u8)> = Mutex::new((0, 0));
    static ref DECRYPT_DATA: Mutex<(usize, u8)> = Mutex::new((0, 0));
}

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
SMSG_BINDPOINTUPDATE 0x155 {
    homebind_x: f32: LittleEndian,
    homebind_y: f32: LittleEndian,
    homebind_z: f32: LittleEndian,
    homebind_map_id: u32: LittleEndian,
    homebind_area_id: u32: LittleEndian,
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

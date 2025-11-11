#![allow(non_camel_case_types)]

use std::{
    ffi::CString,
    io::{ErrorKind, Write},
    sync::Mutex,
};

use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use lazy_static::lazy_static;
use macros::create_server_packets;

use crate::{account_result::AccountResult, character_info::CharacterInfo};

lazy_static! {
    static ref ENCRYPT_DATA: Mutex<(usize, u8)> = Mutex::new((0, 0));
    static ref DECRYPT_DATA: Mutex<(usize, u8)> = Mutex::new((0, 0));
}

create_server_packets!(
SMSG_CHAR_CREATE 0x03A {
    result: AccountResult: LittleEndian,
},
SMSG_CHAR_ENUM 0x03B {
    characters: Vec<CharacterInfo>: LittleEndian,
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

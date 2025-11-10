#![allow(non_camel_case_types)]

use std::{io::Write, sync::Mutex};

use byteorder::{BigEndian, ByteOrder, LittleEndian, WriteBytesExt};
use lazy_static::lazy_static;
use macros::create_server_packets;

use crate::account_result::AccountResult;

lazy_static! {
    static ref ENCRYPT_DATA: Mutex<(usize, u8)> = Mutex::new((0, 0));
    static ref DECRYPT_DATA: Mutex<(usize, u8)> = Mutex::new((0, 0));
}

create_server_packets!(
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

impl<T: ByteOrder> OrderedWrite<T> for u32 {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u32::<T>(*self)
    }
}

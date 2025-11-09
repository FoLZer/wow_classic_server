#![allow(non_camel_case_types)]

use std::{ffi::CString, io::Cursor};

use macros::create_client_packets;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};

create_client_packets!(
CMSG_WORLD_TELEPORT 0x008 {
    time: u32: LittleEndian,
    mapid: u32: LittleEndian,
    position_x: f32: LittleEndian,
    position_y: f32: LittleEndian,
    position_z: f32: LittleEndian,
    orientation: f32: LittleEndian
},
CMSG_CHAR_CREATE 0x036 {
    character_name: CString: LittleEndian,
    race: u8: LittleEndian,
    class: u8: LittleEndian,
    gender: u8: LittleEndian,
    skin: u8: LittleEndian,
    face: u8: LittleEndian,
    hairstyle: u8: LittleEndian,
    haircolor: u8: LittleEndian,
    facialhair: u8: LittleEndian,
    outfit_id: u8: LittleEndian
},
);

pub trait OrderedRead<T: ByteOrder> {
    fn from_reader(reader: &mut Cursor<&[u8]>) -> std::io::Result<Self>
    where
        Self: Sized;
}

impl<T: ByteOrder> OrderedRead<T> for u8 {
    fn from_reader(reader: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        reader.read_u8()
    }
}

impl<T: ByteOrder> OrderedRead<T> for u32 {
    fn from_reader(reader: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        reader.read_u32::<T>()
    }
}

impl<T: ByteOrder> OrderedRead<T> for f32 {
    fn from_reader(reader: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        reader.read_f32::<T>()
    }
}

impl<T: ByteOrder> OrderedRead<T> for CString {
    fn from_reader(reader: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        let mut b = Vec::new();
        loop {
            let d = reader.read_u8()?;
            if d == 0 {
                break;
            }
            b.push(d);
        }
        Ok(unsafe { CString::from_vec_unchecked(b) })
    }
}

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
    //InvalidData,
    InvalidOpcode(u32),
    UnexpectedOpcode,
}

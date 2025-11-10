#![allow(non_camel_case_types)]

use std::{ffi::CString, io::Cursor};

use macros::create_client_packets;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use lazy_static::lazy_static;
use tokio::{io::AsyncRead, sync::Mutex};

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
CMSG_AUTH_SESSION 0x1ED {
    build: u32: LittleEndian,
    server_id: u32: LittleEndian,
    username: CString: BigEndian,
    client_seed: u32: LittleEndian,
    client_proof: [u8; 20]: LittleEndian,
    decompressed_addon_info_size: u32: LittleEndian,
    compressed_addon_info: EndDataVec: LittleEndian
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

pub struct EndDataVec(pub Vec<u8>);

impl<T: ByteOrder> OrderedRead<T> for EndDataVec {
    fn from_reader(reader: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        Ok(Self(reader.split().1.to_vec()))
    }
}

impl<T: ByteOrder, const SIZE: usize> OrderedRead<T> for [u8; SIZE] {
    fn from_reader(reader: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        use std::io::Read;

        let mut b = [0; SIZE];
        reader.read_exact(&mut b).unwrap();
        Ok(b)
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

pub trait ReadablePacket {
    fn from_reader(cursor: &mut ::std::io::Cursor<&[u8]>) -> Result<Self, std::io::Error> where Self: Sized;
    fn opcode() -> u32;
}

lazy_static! {
    static ref DECRYPT_DATA: Mutex<(usize, u8)> = Mutex::new((0, 0));
}

pub async fn read_specific_packet<R: AsyncRead + Unpin, T: ReadablePacket>(
    reader: &mut R,
    session_key: Option<[u8; 40]>,
) -> Result<T, ParseError> {
    use tokio::io::AsyncReadExt;

    let buf = {
        let size = match session_key {
            Some(session_key) => {
                let mut inner_buf = [0; 2];
                reader
                    .read_exact(&mut inner_buf)
                    .await
                    .map_err(ParseError::Io)?;
                let mut lock = DECRYPT_DATA.lock().await;
                for b in &mut inner_buf {
                    let dec = (b.wrapping_sub(lock.1)) ^ session_key[lock.0];
                    lock.0 = (lock.0 + 1) % session_key.len();
                    lock.1 = *b;
                    *b = dec;
                }
                u16::from_be_bytes(inner_buf)
            }
            None => reader.read_u16().await.map_err(ParseError::Io)?,
        };
        let mut buf = vec![0; size as usize];
        reader.read_exact(&mut buf).await.map_err(ParseError::Io)?;
        buf
    };

    let mut cursor = Cursor::new(buf.as_slice());

    let opcode = match session_key {
        Some(session_key) => {
            let mut inner_buf = [0; 4];
            cursor
                .read_exact(&mut inner_buf)
                .await
                .map_err(ParseError::Io)?;
            let mut lock = DECRYPT_DATA.lock().await;
            for b in &mut inner_buf {
                let dec = (b.wrapping_sub(lock.1)) ^ session_key[lock.0];
                lock.0 = (lock.0 + 1) % session_key.len();
                lock.1 = *b;
                *b = dec;
            }
            u32::from_le_bytes(inner_buf)
        }
        None => ReadBytesExt::read_u32::<LittleEndian>(&mut cursor).unwrap(),
    };

    if opcode != T::opcode() {
        return Err(ParseError::UnexpectedOpcode);
    }

    T::from_reader(&mut cursor).map_err(ParseError::Io)
}

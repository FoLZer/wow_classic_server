#![allow(non_camel_case_types)]

use std::{ffi::CString, io::Cursor};

use macros::create_client_packets;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use tokio::io::AsyncRead;

use crate::movement_info::MovementInfo;

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
CMSG_CHAR_ENUM 0x037 {},
CMSG_CHAR_DELETE 0x038 {
    character_guid: u64: LittleEndian
},
CMSG_PLAYER_LOGIN 0x03D {
    character_guid: u64: LittleEndian
},
CMSG_PLAYER_LOGOUT 0x04A {},
CMSG_LOGOUT_REQUEST 0x04B {},
CMSG_LOGOUT_CANCEL 0x04E {},
CMSG_NAME_QUERY 0x050 {
    guid: u64: LittleEndian,
},
CMSG_ITEM_QUERY_SINGLE 0x056 {
    item_id: u32: LittleEndian,
    guid: u64: LittleEndian
},
CMSG_ADD_FRIEND 0x069 {
    friend_name: CString: LittleEndian
},
CMSG_MESSAGECHAT 0x095 {
    ty: u32: LittleEndian,
    lang: u32: LittleEndian,
    data: EndDataVec: LittleEndian
},
CMSG_JOIN_CHANNEL 0x097 {
    channel_name: CString: LittleEndian,
    pass: CString: LittleEndian
},
MSG_MOVE_START_FORWARD 0x0B5 {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_BACKWARD 0x0B6 {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_STOP 0x0B7 {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_STRAFE_LEFT 0x0B8 {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_STRAFE_RIGHT 0x0B9 {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_STOP_STRAFE 0x0BA {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_JUMP 0x0BB {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_TURN_LEFT 0x0BC {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_TURN_RIGHT 0x0BD {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_STOP_TURN 0x0BE {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_PITCH_UP 0x0BF {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_PITCH_DOWN 0x0C1 {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_STOP_PITCH 0x0C2 {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_SET_RUN_MODE 0x0C3 {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_SET_WALK_MODE 0x0C4 {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_FALL_LAND 0x0C9 {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_START_SWIM 0x0CA {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_STOP_SWIM 0x0CB {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_SET_FACING 0x0DA {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_SET_PITCH 0x0DB {
    movement_info: MovementInfo: LittleEndian,
},
MSG_MOVE_HEARTBEAT 0x0EE {
    movement_info: MovementInfo: LittleEndian,
},
CMSG_TUTORIAL_FLAG 0x0FE {
    i_flag: u32: LittleEndian
},
CMSG_TUTORIAL_CLEAR 0x0FF {},
CMSG_TUTORIAL_RESET 0x100 {},
CMSG_STANDSTATECHANGE 0x101 {
    anim_state: u32: LittleEndian
},
CMSG_SWAP_INV_ITEM 0x10D {
    src_slot: u8: LittleEndian,
    dst_slot: u8: LittleEndian,
},
CMSG_QUERY_TIME 0x1CE {},
CMSG_PING 0x1DC {
    sequence_id: u32: LittleEndian,
    latency: u32: LittleEndian
},
CMSG_SETSHEATHED 0x1E0 {
    sheath_state: u32: LittleEndian
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
CMSG_ZONEUPDATE 0x1F4 {
    new_zone: u32: LittleEndian
},
CMSG_UPDATE_ACCOUNT_DATA 0x20B {
    data: EndDataVec: LittleEndian
},
CMSG_GMTICKET_GETTICKET 0x211 {},
CMSG_SET_ACTIVE_MOVER 0x26A {
    guid: u64: LittleEndian
},
MSG_QUERY_NEXT_MAIL_TIME 0x284 {},
CMSG_MEETINGSTONE_INFO 0x296 {},
CMSG_MOVE_FALL_RESET 0x2CA {
    movement_info: MovementInfo: LittleEndian,
},
CMSG_REQUEST_RAID_INFO 0x2CD {},
CMSG_MOVE_TIME_SKIPPED 0x2CE {
    guid: u64: LittleEndian,
    time_skipped: u32: LittleEndian
},
CMSG_BATTLEFIELD_STATUS 0x2D3 {},
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

impl<T: ByteOrder> OrderedRead<T> for u64 {
    fn from_reader(reader: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        reader.read_u64::<T>()
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

pub enum ParseError {
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
    //InvalidData,
    InvalidOpcode(u32),
    UnexpectedOpcode,
}

impl core::fmt::Debug for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            ParseError::Io(f0) => f.debug_tuple("Io").field(&f0).finish(),
            ParseError::Utf8(f0) => f.debug_tuple("Utf8").field(&f0).finish(),
            ParseError::InvalidOpcode(f0) => f
                .debug_tuple("InvalidOpcode")
                .field(&format_args!("{:X}", &f0))
                .finish(),
            ParseError::UnexpectedOpcode => f.write_str("UnexpectedOpcode"),
        }
    }
}

pub trait ReadablePacket {
    fn from_reader(cursor: &mut ::std::io::Cursor<&[u8]>) -> Result<Self, std::io::Error>
    where
        Self: Sized;
    fn opcode() -> u32;
}

pub async fn read_specific_packet<R: AsyncRead + Unpin, T: ReadablePacket>(
    reader: &mut R,
    session_key: Option<[u8; 40]>,
    decrypt_data: &mut (usize, u8),
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

                for b in &mut inner_buf {
                    let dec = (b.wrapping_sub(decrypt_data.1)) ^ session_key[decrypt_data.0];
                    decrypt_data.0 = (decrypt_data.0 + 1) % session_key.len();
                    decrypt_data.1 = *b;
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

            for b in &mut inner_buf {
                let dec = (b.wrapping_sub(decrypt_data.1)) ^ session_key[decrypt_data.0];
                decrypt_data.0 = (decrypt_data.0 + 1) % session_key.len();
                decrypt_data.1 = *b;
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

// There's never a need to decode a random packet without a session key
pub async fn read_packet<R: AsyncRead + Unpin>(
    reader: &mut R,
    session_key: [u8; 40],
    decrypt_data: &mut (usize, u8),
) -> Result<ClientPacket, ParseError> {
    use tokio::io::AsyncReadExt;

    let packet_buf = {
        let mut size_buf = [0; 2];
        reader
            .read_exact(&mut size_buf)
            .await
            .map_err(ParseError::Io)?;
        for b in &mut size_buf {
            let dec = (b.wrapping_sub(decrypt_data.1)) ^ session_key[decrypt_data.0];
            decrypt_data.0 = (decrypt_data.0 + 1) % session_key.len();
            decrypt_data.1 = *b;
            *b = dec;
        }
        let size = u16::from_be_bytes(size_buf);
        let mut buf = vec![0; size as usize];
        reader.read_exact(&mut buf).await.map_err(ParseError::Io)?;
        buf
    };

    let mut cursor = Cursor::new(packet_buf.as_slice());

    let mut opcode_buf = [0; 4];
    cursor
        .read_exact(&mut opcode_buf)
        .await
        .map_err(ParseError::Io)?;
    for b in &mut opcode_buf {
        let dec = (b.wrapping_sub(decrypt_data.1)) ^ session_key[decrypt_data.0];
        decrypt_data.0 = (decrypt_data.0 + 1) % session_key.len();
        decrypt_data.1 = *b;
        *b = dec;
    }
    let opcode = u32::from_le_bytes(opcode_buf);

    decode_packet_inner(&mut cursor, opcode)
}

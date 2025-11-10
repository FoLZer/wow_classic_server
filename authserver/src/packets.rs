use std::{ffi::CString, net::SocketAddr};

use byteorder::LittleEndian;
use ipc_comms::realm_types::{RealmCategory, RealmType};

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum ClientPacket {
    CMD_AUTH_LOGON_CHALLENGE {
        protocol_version: u8,
        game_name: [u8; 4],
        version: [u8; 3],
        build: u16,
        platform: [u8; 4],
        os: [u8; 4],
        locale: [u8; 4],
        worldregion_bias: u32,
        ip: u32,
        account_name: String,
    },
    CMD_AUTH_LOGON_PROOF {
        client_public_key: [u8; 32],
        client_proof: [u8; 20],
        crc_hash: [u8; 20],
        keys_telemetry: Vec<KeyStruct>,
        pin_two_factor_data: Option<PinClientStruct>,
    },
    CMD_AUTH_RECONNECT_CHALLENGE,
    CMD_AUTH_RECONNECT_PROOF,
    CMD_SURVEY_RESULT,
    CMD_REALM_LIST,
    CMD_XFER_ACCEPT,
    CMD_XFER_RESUME,
    CMD_XFER_CANCEL,
}

#[allow(unused)]
pub mod opcodes {
    pub const CMD_AUTH_LOGON_CHALLENGE: u8 = 0x00;
    pub const CMD_AUTH_LOGON_PROOF: u8 = 0x01;
    pub const CMD_AUTH_RECONNECT_CHALLENGE: u8 = 0x02;
    pub const CMD_AUTH_RECONNECT_PROOF: u8 = 0x03;
    pub const CMD_SURVEY_RESULT: u8 = 0x04;
    pub const CMD_REALM_LIST: u8 = 0x10;
    pub const CMD_XFER_INITIATE: u8 = 0x30;
    pub const CMD_XFER_DATA: u8 = 0x31;
    pub const CMD_XFER_ACCEPT: u8 = 0x32;
    pub const CMD_XFER_RESUME: u8 = 0x33;
    pub const CMD_XFER_CANCEL: u8 = 0x34;
}

impl ClientPacket {
    pub async fn from_reader<R: tokio::io::AsyncReadExt + std::marker::Unpin>(
        reader: &mut R,
        expected_packet: Option<u8>,
    ) -> Result<Self, ParseError> {
        let opcode = reader.read_u8().await?;
        if let Some(op) = expected_packet {
            if opcode != op {
                return Err(ParseError::UnexpectedOpcode(opcode));
            }
        }
        match opcode {
            0x00 => Ok(Self::CMD_AUTH_LOGON_CHALLENGE {
                protocol_version: reader.read_u8().await?,
                game_name: {
                    let _ = reader.read_u16().await?; //size, unused
                    let mut b = [0; 4];
                    reader.read_exact(&mut b).await?;
                    b
                },
                version: {
                    let mut b = [0; 3];
                    reader.read_exact(&mut b).await?;
                    b
                },
                build: reader.read_u16_le().await?,
                platform: {
                    let mut b = [0; 4];
                    reader.read_exact(&mut b).await?;
                    b
                },
                os: {
                    let mut b = [0; 4];
                    reader.read_exact(&mut b).await?;
                    b
                },
                locale: {
                    let mut b = [0; 4];
                    reader.read_exact(&mut b).await?;
                    b
                },
                worldregion_bias: reader.read_u32_le().await?,
                ip: reader.read_u32().await?,
                account_name: {
                    let size = reader.read_u8().await?;
                    if size > 98 {
                        return Err(ParseError::InvalidData); // Too many bytes, not accepted.
                    }
                    let mut buf = vec![0; size as usize];
                    reader.read_exact(&mut buf).await?;
                    String::from_utf8(buf)?
                },
            }),
            0x01 => Ok(Self::CMD_AUTH_LOGON_PROOF {
                client_public_key: {
                    let mut b = [0; 32];
                    reader.read_exact(&mut b).await?;
                    b
                },
                client_proof: {
                    let mut b = [0; 20];
                    reader.read_exact(&mut b).await?;
                    b
                },
                crc_hash: {
                    let mut b = [0; 20];
                    reader.read_exact(&mut b).await?;
                    b
                },
                keys_telemetry: {
                    let num_keys = reader.read_u8().await?;
                    let mut v = Vec::with_capacity(num_keys as usize);
                    for _ in 0..num_keys {
                        v.push(KeyStruct {
                            unk1: reader.read_u16_le().await?,
                            unk2: reader.read_u32_le().await?,
                            unk3: {
                                let mut b = [0; 4];
                                reader.read_exact(&mut b).await?;
                                b
                            },
                            cd_key_proof: {
                                let mut b = [0; 20];
                                reader.read_exact(&mut b).await?;
                                b
                            },
                        });
                    }
                    v
                },
                pin_two_factor_data: {
                    let two_factor_enabled = reader.read_u8().await? != 0;
                    if two_factor_enabled {
                        Some(PinClientStruct {
                            pin_salt: {
                                let mut b = [0; 16];
                                reader.read_exact(&mut b).await?;
                                b
                            },
                            pin_hash: {
                                let mut b = [0; 20];
                                reader.read_exact(&mut b).await?;
                                b
                            },
                        })
                    } else {
                        None
                    }
                },
            }),
            0x02 => Ok(Self::CMD_AUTH_RECONNECT_CHALLENGE),
            0x03 => Ok(Self::CMD_AUTH_RECONNECT_PROOF),
            0x04 => Ok(Self::CMD_SURVEY_RESULT),
            0x10 => {
                let _ = reader.read_u32().await.unwrap(); //padding
                Ok(Self::CMD_REALM_LIST)
            }
            0x32 => Ok(Self::CMD_XFER_ACCEPT),
            0x33 => Ok(Self::CMD_XFER_RESUME),
            0x34 => Ok(Self::CMD_XFER_CANCEL),
            _ => Err(ParseError::InvalidOpcode),
        }
    }
}

#[derive(Debug)]
#[allow(unused)]
pub struct PinClientStruct {
    pin_salt: [u8; 16],
    pin_hash: [u8; 20],
}

#[derive(Debug)]
#[allow(unused)]
pub struct KeyStruct {
    unk1: u16,
    unk2: u32,
    unk3: [u8; 4],
    cd_key_proof: [u8; 20],
}

#[allow(unused)]
#[derive(Debug)]
pub enum ParseError {
    IoError(std::io::Error),
    Utf8Error(std::string::FromUtf8Error),
    InvalidData,
    InvalidOpcode,
    UnexpectedOpcode(u8),
}

impl From<std::io::Error> for ParseError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<std::string::FromUtf8Error> for ParseError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        Self::Utf8Error(value)
    }
}

#[allow(non_camel_case_types)]
#[allow(unused)]
#[derive(Debug)]
pub enum ServerPacket {
    CMD_AUTH_LOGON_CHALLENGE {
        server_public_key: [u8; 32],
        generator: Vec<u8>,
        large_safe_prime: Vec<u8>,
        salt: [u8; 32],
        crc_salt: [u8; 16],
        pin: Option<PinStruct>,
    },
    CMD_AUTH_LOGON_ERROR {
        error: LoginPacketErrors,
    },
    CMD_AUTH_LOGON_PROOF {
        server_proof: [u8; 20],
        hardware_survey_id: u32,
    },
    CMD_AUTH_RECONNECT_CHALLENGE,
    CMD_AUTH_RECONNECT_PROOF,
    CMD_REALM_LIST {
        realms: Vec<RealmInfo>,
    },
    CMD_XFER_INITIATE,
    CMD_XFER_DATA,
}

#[derive(Debug)]
pub struct RealmInfo {
    pub realm_type: RealmType,
    pub flags: u8,
    pub realm_name: CString,
    pub address_port: SocketAddr,
    pub population: f32,
    pub num_chars: u8,
    pub realm_category: RealmCategory,
    pub realm_id: u8,
}

#[derive(Debug)]
pub struct PinStruct {
    pin_grid_seed: u32,
    pin_salt: [u8; 16],
}

impl ServerPacket {
    pub fn to_bytes(&self) -> Vec<u8> {
        use byteorder::WriteBytesExt;
        use std::io::Write;

        let mut buf = Vec::new();
        match self {
            ServerPacket::CMD_AUTH_LOGON_CHALLENGE {
                server_public_key,
                generator,
                large_safe_prime,
                salt,
                crc_salt,
                pin,
            } => {
                buf.write_u8(0x00).unwrap(); //opcode
                buf.write_u8(0x00).unwrap(); //protocol_version
                buf.write_u8(0x00).unwrap(); //result, SUCCESS
                buf.write_all(server_public_key).unwrap();
                buf.write_u8(generator.len() as u8).unwrap();
                buf.write_all(generator).unwrap();
                buf.write_u8(large_safe_prime.len() as u8).unwrap();
                buf.write_all(large_safe_prime).unwrap();
                buf.write_all(salt).unwrap();
                buf.write_all(crc_salt).unwrap();
                buf.write_u8(pin.is_some() as u8).unwrap();
                match pin {
                    Some(pin) => {
                        buf.write_u32::<LittleEndian>(pin.pin_grid_seed).unwrap();
                        buf.write_all(&pin.pin_salt).unwrap();
                    }
                    None => (),
                }
            }
            ServerPacket::CMD_AUTH_LOGON_ERROR { error: _ } => todo!(),
            ServerPacket::CMD_AUTH_LOGON_PROOF {
                server_proof,
                hardware_survey_id,
            } => {
                buf.write_u8(0x01).unwrap(); //opcode
                buf.write_u8(0x00).unwrap(); //result, SUCCESS
                buf.write_all(server_proof).unwrap();
                buf.write_u32::<LittleEndian>(*hardware_survey_id).unwrap();
            }
            ServerPacket::CMD_AUTH_RECONNECT_CHALLENGE => todo!(),
            ServerPacket::CMD_AUTH_RECONNECT_PROOF => todo!(),
            ServerPacket::CMD_REALM_LIST { realms } => {
                buf.write_u8(0x10).unwrap(); //opcode
                let mut size: u16 = 4 + 1 + 2;
                for realm in realms {
                    size += 4
                        + 1
                        + realm.realm_name.as_bytes_with_nul().len() as u16
                        + CString::new(realm.address_port.to_string())
                            .unwrap()
                            .as_bytes_with_nul()
                            .len() as u16
                        + 4
                        + 1
                        + 1
                        + 1;
                }
                buf.write_u16::<LittleEndian>(size).unwrap();
                buf.write_u32::<LittleEndian>(0).unwrap(); //padding
                buf.write_u8(realms.len() as u8).unwrap();
                for realm in realms {
                    buf.write_u32::<LittleEndian>(realm.realm_type.clone().into())
                        .unwrap();
                    buf.write_u8(realm.flags).unwrap();
                    buf.write_all(realm.realm_name.as_bytes_with_nul()).unwrap();
                    buf.write_all(
                        CString::new(realm.address_port.to_string())
                            .unwrap()
                            .as_bytes_with_nul(),
                    )
                    .unwrap();
                    buf.write_f32::<LittleEndian>(realm.population).unwrap();
                    buf.write_u8(realm.num_chars).unwrap();
                    buf.write_u8(realm.realm_category.clone().into()).unwrap();
                    buf.write_u8(realm.realm_id).unwrap();
                }
                buf.write_u8(2).unwrap();
                buf.write_u8(0).unwrap();
                //buf.write_u16::<LittleEndian>(0).unwrap();
            }
            ServerPacket::CMD_XFER_INITIATE => todo!(),
            ServerPacket::CMD_XFER_DATA => todo!(),
        }
        buf
    }
}

#[allow(non_camel_case_types)]
#[allow(unused)]
#[derive(Debug)]
pub enum LoginPacketErrors {
    FAIL_UNKNOWN0,
    FAIL_UNKNOWN1,
    FAIL_BANNED,
    FAIL_UNKNOWN_ACCOUNT,
    FAIL_INCORRECT_PASSWORD,
    FAIL_ALREADY_ONLINE,
    FAIL_NO_TIME,
    FAIL_DB_BUSY,
    FAIL_VERSION_INVALID,
    LOGIN_DOWNLOAD_FILE,
    FAIL_INVALID_SERVER,
    FAIL_SUSPENDED,
    FAIL_NO_ACCESS,
    SUCCESS_SURVEY,
    FAIL_PARENTALCONTROL,
    //FAIL_LOCKED_ENFORCED
}

impl From<LoginPacketErrors> for u8 {
    fn from(val: LoginPacketErrors) -> Self {
        match val {
            LoginPacketErrors::FAIL_UNKNOWN0 => 0x01,
            LoginPacketErrors::FAIL_UNKNOWN1 => 0x02,
            LoginPacketErrors::FAIL_BANNED => 0x03,
            LoginPacketErrors::FAIL_UNKNOWN_ACCOUNT => 0x04,
            LoginPacketErrors::FAIL_INCORRECT_PASSWORD => 0x05,
            LoginPacketErrors::FAIL_ALREADY_ONLINE => 0x06,
            LoginPacketErrors::FAIL_NO_TIME => 0x07,
            LoginPacketErrors::FAIL_DB_BUSY => 0x08,
            LoginPacketErrors::FAIL_VERSION_INVALID => 0x09,
            LoginPacketErrors::LOGIN_DOWNLOAD_FILE => 0x0A,
            LoginPacketErrors::FAIL_INVALID_SERVER => 0x0B,
            LoginPacketErrors::FAIL_SUSPENDED => 0x0C,
            LoginPacketErrors::FAIL_NO_ACCESS => 0x0D,
            LoginPacketErrors::SUCCESS_SURVEY => 0x0E,
            LoginPacketErrors::FAIL_PARENTALCONTROL => 0x0F,
            //LoginPacketErrors::FAIL_LOCKED_ENFORCED => 0x10
        }
    }
}

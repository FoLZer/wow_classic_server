use std::{
    ffi::CString,
    io::{Cursor, Error, ErrorKind},
};

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use common::guid::{self, Guid, LivingGuid};

use crate::{client::OrderedRead, server::OrderedWrite};

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum ChatType {
    Say,
    Party,
    Raid,
    Guild,
    Officer,
    Yell,
    Whisper { target_name: CString },
    Emote,
    Channel { channel_name: CString },
    AFK,
    DND,
    RaidLeader,
    RaidWarning,
    BattleGround,
    BattleGroundLeader,
}

impl ChatType {
    fn read<T: ByteOrder>(ty: u32, reader: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        Ok(match ty {
            0x00 => Self::Say,
            0x01 => Self::Party,
            0x02 => Self::Raid,
            0x03 => Self::Guild,
            0x04 => Self::Officer,
            0x05 => Self::Yell,
            0x06 => Self::Whisper {
                target_name: <CString as OrderedRead<T>>::from_reader(reader)?,
            },
            0x08 => Self::Emote,
            0x0E => Self::Channel {
                channel_name: <CString as OrderedRead<T>>::from_reader(reader)?,
            },
            0x14 => Self::AFK,
            0x15 => Self::DND,
            0x57 => Self::RaidLeader,
            0x58 => Self::RaidWarning,
            0x5C => Self::BattleGround,
            0x5D => Self::BattleGroundLeader,
            v => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("unknown chat type {v:#x}"),
                ));
            }
        })
    }
}

/// Body of CMSG_MESSAGECHAT.
#[derive(Debug, Clone)]
pub struct ClientChatMessage {
    pub ty: ChatType,
    pub lang: u32,
    pub message: CString,
}

impl ClientChatMessage {
    const LANG_ADDON: u32 = 0xFFFFFFFF;

    pub fn is_addon_message(&self) -> bool {
        self.lang == Self::LANG_ADDON
    }
}

impl<T: ByteOrder> OrderedRead<T> for ClientChatMessage {
    fn from_reader(reader: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        let ty = reader.read_u32::<LittleEndian>()?;
        let lang = reader.read_u32::<LittleEndian>()?;
        let ty = ChatType::read::<T>(ty, reader)?;
        let message = <CString as OrderedRead<T>>::from_reader(reader)?;
        Ok(Self { ty, lang, message })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChatTag {
    None = 0,
    AFK = 1,
    DND = 2,
    /// Also bypasses the receiver's ignore list
    GM = 3,
}

/// Chat type of SMSG_MESSAGECHAT together with its type-specific fields.
/// Types without `lang` are sent as universal, types without `tag` as `ChatTag::None`:
/// the client ignores those fields for them.
#[derive(Debug, Clone)]
pub enum ServerChatType {
    Say {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    Party {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    Raid {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    Guild {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    Officer {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    Yell {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    Whisper {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    /// Echo of a whisper sent back to its author ("To X: ...")
    WhisperInform {
        receiver: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    Emote {
        sender: Guid<guid::Player>,
        tag: ChatTag,
    },
    System,
    MonsterSay {
        speaker: Guid<guid::Unit>,
        name: CString,
        target: Option<LivingGuid>,
        lang: u32,
    },
    MonsterYell {
        speaker: Guid<guid::Unit>,
        name: CString,
        target: Option<LivingGuid>,
        lang: u32,
    },
    MonsterEmote {
        name: CString,
        target: Option<LivingGuid>,
    },
    Channel {
        channel_name: CString,
        sender_pvp_rank: u32,
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    /// Auto-reply to a whisper sent to an AFK player
    AFK {
        sender: Guid<guid::Player>,
        tag: ChatTag,
    },
    /// Auto-reply to a whisper sent to a DND player
    DND {
        sender: Guid<guid::Player>,
        tag: ChatTag,
    },
    MonsterWhisper {
        name: CString,
        target: Option<LivingGuid>,
        lang: u32,
    },
    RaidLeader {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    RaidWarning {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    RaidBossEmote {
        name: CString,
        target: Option<LivingGuid>,
    },
    BattleGround {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
    BattleGroundLeader {
        sender: Guid<guid::Player>,
        lang: u32,
        tag: ChatTag,
    },
}

impl ServerChatType {
    pub fn id(&self) -> u8 {
        match self {
            ServerChatType::Say { .. } => 0x00,
            ServerChatType::Party { .. } => 0x01,
            ServerChatType::Raid { .. } => 0x02,
            ServerChatType::Guild { .. } => 0x03,
            ServerChatType::Officer { .. } => 0x04,
            ServerChatType::Yell { .. } => 0x05,
            ServerChatType::Whisper { .. } => 0x06,
            ServerChatType::WhisperInform { .. } => 0x07,
            ServerChatType::Emote { .. } => 0x08,
            ServerChatType::System => 0x0A,
            ServerChatType::MonsterSay { .. } => 0x0B,
            ServerChatType::MonsterYell { .. } => 0x0C,
            ServerChatType::MonsterEmote { .. } => 0x0D,
            ServerChatType::Channel { .. } => 0x0E,
            ServerChatType::AFK { .. } => 0x14,
            ServerChatType::DND { .. } => 0x15,
            ServerChatType::MonsterWhisper { .. } => 0x1A,
            ServerChatType::RaidLeader { .. } => 0x57,
            ServerChatType::RaidWarning { .. } => 0x58,
            ServerChatType::RaidBossEmote { .. } => 0x5A,
            ServerChatType::BattleGround { .. } => 0x5C,
            ServerChatType::BattleGroundLeader { .. } => 0x5D,
        }
    }
}

/// Body of SMSG_MESSAGECHAT.
#[derive(Debug, Clone)]
pub struct ServerChatMessage {
    pub ty: ServerChatType,
    pub text: CString,
}

impl ServerChatMessage {
    const LANG_UNIVERSAL: u32 = 0;
}

impl<T: ByteOrder> OrderedWrite<T> for ServerChatMessage {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u8(self.ty.id())?;
        let tag = match &self.ty {
            // The first guid picks the unit that shows the chat bubble
            ServerChatType::Say { sender, lang, tag }
            | ServerChatType::Party { sender, lang, tag }
            | ServerChatType::Yell { sender, lang, tag } => {
                writer.write_u32::<LittleEndian>(*lang)?;
                OrderedWrite::<LittleEndian>::write(sender, writer)?;
                OrderedWrite::<LittleEndian>::write(sender, writer)?;
                *tag
            }
            ServerChatType::Raid { sender, lang, tag }
            | ServerChatType::Guild { sender, lang, tag }
            | ServerChatType::Officer { sender, lang, tag }
            | ServerChatType::Whisper { sender, lang, tag }
            | ServerChatType::RaidLeader { sender, lang, tag }
            | ServerChatType::RaidWarning { sender, lang, tag }
            | ServerChatType::BattleGround { sender, lang, tag }
            | ServerChatType::BattleGroundLeader { sender, lang, tag }
            | ServerChatType::WhisperInform {
                receiver: sender,
                lang,
                tag,
            } => {
                writer.write_u32::<LittleEndian>(*lang)?;
                OrderedWrite::<LittleEndian>::write(sender, writer)?;
                *tag
            }
            ServerChatType::Emote { sender, tag }
            | ServerChatType::AFK { sender, tag }
            | ServerChatType::DND { sender, tag } => {
                writer.write_u32::<LittleEndian>(Self::LANG_UNIVERSAL)?;
                OrderedWrite::<LittleEndian>::write(sender, writer)?;
                *tag
            }
            ServerChatType::Channel {
                channel_name,
                sender_pvp_rank,
                sender,
                lang,
                tag,
            } => {
                writer.write_u32::<LittleEndian>(*lang)?;
                OrderedWrite::<LittleEndian>::write(channel_name, writer)?;
                writer.write_u32::<LittleEndian>(*sender_pvp_rank)?;
                OrderedWrite::<LittleEndian>::write(sender, writer)?;
                *tag
            }
            ServerChatType::System => {
                writer.write_u32::<LittleEndian>(Self::LANG_UNIVERSAL)?;
                writer.write_u64::<LittleEndian>(0)?;
                ChatTag::None
            }
            ServerChatType::MonsterSay {
                speaker,
                name,
                target,
                lang,
            }
            | ServerChatType::MonsterYell {
                speaker,
                name,
                target,
                lang,
            } => {
                writer.write_u32::<LittleEndian>(*lang)?;
                OrderedWrite::<LittleEndian>::write(speaker, writer)?;
                // Length includes the NUL: the client reads the bytes as a C string
                writer.write_u32::<LittleEndian>(name.as_bytes_with_nul().len() as u32)?;
                OrderedWrite::<LittleEndian>::write(name, writer)?;
                writer.write_u64::<LittleEndian>(target.map_or(0, |v| v.get().get()))?;
                ChatTag::None
            }
            ServerChatType::MonsterWhisper { name, target, lang } => {
                writer.write_u32::<LittleEndian>(*lang)?;
                writer.write_u32::<LittleEndian>(name.as_bytes_with_nul().len() as u32)?;
                OrderedWrite::<LittleEndian>::write(name, writer)?;
                writer.write_u64::<LittleEndian>(target.map_or(0, |v| v.get().get()))?;
                ChatTag::None
            }
            ServerChatType::MonsterEmote { name, target }
            | ServerChatType::RaidBossEmote { name, target } => {
                writer.write_u32::<LittleEndian>(Self::LANG_UNIVERSAL)?;
                writer.write_u32::<LittleEndian>(name.as_bytes_with_nul().len() as u32)?;
                OrderedWrite::<LittleEndian>::write(name, writer)?;
                writer.write_u64::<LittleEndian>(target.map_or(0, |v| v.get().get()))?;
                ChatTag::None
            }
        };
        writer.write_u32::<LittleEndian>(self.text.as_bytes_with_nul().len() as u32)?;
        OrderedWrite::<LittleEndian>::write(&self.text, writer)?;
        writer.write_u8(tag as u8)
    }
}

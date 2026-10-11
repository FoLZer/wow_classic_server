use std::ffi::CString;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};

use crate::server::OrderedWrite;

pub struct GossipMenuItem {
    pub index: u32,
    pub icon: u8,
    pub coded: u8,
    pub text: CString,
}

impl<T: ByteOrder> OrderedWrite<T> for GossipMenuItem {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <u32 as OrderedWrite<LittleEndian>>::write(&self.index, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.icon, writer)?;
        <u8 as OrderedWrite<LittleEndian>>::write(&self.coded, writer)?;
        <CString as OrderedWrite<LittleEndian>>::write(&self.text, writer)?;

        Ok(())
    }
}

pub struct GossipQuestMenuItem {
    pub quest_id: u32,
    pub icon: u32,
    pub quest_level: u32,
    pub text: CString,
}

impl<T: ByteOrder> OrderedWrite<T> for GossipQuestMenuItem {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <u32 as OrderedWrite<LittleEndian>>::write(&self.quest_id, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.icon, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.quest_level, writer)?;
        <CString as OrderedWrite<LittleEndian>>::write(&self.text, writer)?;

        Ok(())
    }
}

pub struct U32Counted<T>(pub Vec<T>);

impl<B: ByteOrder, T: OrderedWrite<B>> OrderedWrite<B> for U32Counted<T> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u32::<LittleEndian>(self.0.len().min(u32::MAX as usize) as u32)?;
        for v in &self.0 {
            v.write(writer)?;
        }

        Ok(())
    }
}

pub struct NpcTextOption {
    pub probability: f32,
    pub text_male: CString,
    pub text_female: CString,
    pub language: u32,
    /// (emote delay, emote id)
    pub emotes: [(u32, u32); 3],
}

impl NpcTextOption {
    pub fn empty() -> Self {
        Self {
            probability: 0.0,
            text_male: CString::default(),
            text_female: CString::default(),
            language: 0,
            emotes: [(0, 0); 3],
        }
    }
}

impl<T: ByteOrder> OrderedWrite<T> for NpcTextOption {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <f32 as OrderedWrite<LittleEndian>>::write(&self.probability, writer)?;
        <CString as OrderedWrite<LittleEndian>>::write(&self.text_male, writer)?;
        <CString as OrderedWrite<LittleEndian>>::write(&self.text_female, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.language, writer)?;
        for (delay, emote) in &self.emotes {
            <u32 as OrderedWrite<LittleEndian>>::write(delay, writer)?;
            <u32 as OrderedWrite<LittleEndian>>::write(emote, writer)?;
        }

        Ok(())
    }
}

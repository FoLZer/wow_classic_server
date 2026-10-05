use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use common::guid::{Guid, GuidType, LivingGuid};

use crate::server::OrderedWrite;

pub struct Packed<T>(pub T);

impl<T> Packed<T> {
    fn write_packed(writer: &mut Vec<u8>, v: u64) -> std::io::Result<()> {
        let bytes = v.to_le_bytes();
        let mask = bytes.iter().enumerate().fold(0_u8, |mask, (index, byte)| {
            mask | (u8::from(*byte != 0) << index)
        });

        writer.write_u8(mask)?;
        for byte in bytes.into_iter().filter(|byte| *byte != 0) {
            writer.write_u8(byte)?;
        }

        Ok(())
    }
}

impl<T: GuidType> From<Guid<T>> for Packed<Guid<T>> {
    fn from(value: Guid<T>) -> Self {
        Self(value)
    }
}

impl<B: ByteOrder, T: GuidType> OrderedWrite<B> for Packed<Guid<T>> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        Self::write_packed(writer, self.0.get().get())
    }
}

impl<B: ByteOrder, T: GuidType> OrderedWrite<B> for Option<Packed<Guid<T>>> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        if let Some(v) = self {
            OrderedWrite::<B>::write(v, writer)
        } else {
            OrderedWrite::<B>::write(&0u8, writer)
        }
    }
}

impl From<LivingGuid> for Packed<LivingGuid> {
    fn from(value: LivingGuid) -> Self {
        Self(value)
    }
}

impl<B: ByteOrder> OrderedWrite<B> for Packed<LivingGuid> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        Self::write_packed(writer, self.0.get().get())
    }
}

impl<T: ByteOrder> OrderedWrite<T> for LivingGuid {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u64::<LittleEndian>(self.get().get())
    }
}

impl<B: ByteOrder> OrderedWrite<B> for Option<Packed<LivingGuid>> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        if let Some(v) = self {
            OrderedWrite::<B>::write(v, writer)
        } else {
            OrderedWrite::<B>::write(&0u8, writer)
        }
    }
}

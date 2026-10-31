use std::num::NonZeroU32;

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};

use crate::{bounded::Bounded, server::OrderedWrite};

pub enum VendorQueryResult {
    Ok {
        /// Count must not be 0 here, if vendor has no items use Err(VendorQueryError::NoInventory)
        items: VendorItems,
    },
    Err(VendorQueryError),
}

impl<B: ByteOrder> OrderedWrite<B> for VendorQueryResult {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        match self {
            VendorQueryResult::Ok { items } => {
                assert_ne!(items.len(), 0);
                writer.write_u8(items.len() as u8)?;
                for item in items {
                    <SerializedVendorItem as OrderedWrite<LittleEndian>>::write(item, writer)?;
                }
            }
            VendorQueryResult::Err(e) => {
                writer.write_u8(0)?;
                writer.write_u8(e.as_u8())?;
            }
        }

        Ok(())
    }
}

pub type VendorItems = Bounded<Vec<SerializedVendorItem>, 128>;

pub enum VendorQueryError {
    NoInventory,
    VendorHostile,
    TooFarAway,
    VendorDead,
    PlayerDead,
}

impl VendorQueryError {
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::NoInventory => 0,
            Self::VendorHostile => 1,
            Self::TooFarAway => 2,
            Self::VendorDead => 3,
            Self::PlayerDead => 4,
        }
    }
}

pub struct SerializedVendorItem {
    pub index: NonZeroU32,
    pub item_id: u32,
    pub display_id: u32,
    pub num_available: Option<u32>,
    pub price: u32,
    pub durability: u32,
    pub purchase_amount: u32,
}

impl<B: ByteOrder> OrderedWrite<B> for SerializedVendorItem {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <u32 as OrderedWrite<LittleEndian>>::write(&self.index.get(), writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.item_id, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.display_id, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(
            &self.num_available.unwrap_or(u32::MAX),
            writer,
        )?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.price, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.durability, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.purchase_amount, writer)?;

        Ok(())
    }
}

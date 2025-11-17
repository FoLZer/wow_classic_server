use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use common::guid::{self, Guid};

use crate::server::OrderedWrite;

pub enum InventoryChangeResult {
    Ok,
    LevelTooLow {
        required_level: u32,
    },
    OtherError {
        error: InventoryChangeError,
        item1: Option<Guid<guid::Item>>,
        item2: Option<Guid<guid::Item>>,
    },
}

impl<T: ByteOrder> OrderedWrite<T> for InventoryChangeResult {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        match self {
            InventoryChangeResult::Ok => {
                writer.write_u8(0)?; //EQUIP_ERR_OK
            }
            InventoryChangeResult::LevelTooLow { required_level } => {
                writer.write_u8(1)?; //EQUIP_ERR_CANT_EQUIP_LEVEL_I
                writer.write_u32::<LittleEndian>(*required_level)?;
            }
            InventoryChangeResult::OtherError {
                error,
                item1,
                item2,
            } => {
                let msg = match error {
                    InventoryChangeError::SlotIsEmpty => 22,
                };
                writer.write_u8(msg)?;
                writer.write_u64::<LittleEndian>(item1.map_or(0, |v| v.get().get()))?;
                writer.write_u64::<LittleEndian>(item2.map_or(0, |v| v.get().get()))?;
                writer.write_u8(0)?; //unkn
            }
        }

        Ok(())
    }
}

pub enum InventoryChangeError {
    SlotIsEmpty, //EQUIP_ERR_SLOT_IS_EMPTY
}

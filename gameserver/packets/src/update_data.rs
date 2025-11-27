use bit_vec::BitVec;
use bitfield_struct::bitfield;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use common::guid::AnyGuid;

use crate::{movement_info::MovementInfo, server::OrderedWrite};

pub struct UpdateBlocks {
    pub has_transport: bool,
    pub blocks: Vec<UpdateData>,
}

impl<T: ByteOrder> OrderedWrite<T> for UpdateBlocks {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        debug_assert!(self.blocks.len() <= u32::MAX as usize); //Should never happen in any realistic scenario
        writer.write_u32::<LittleEndian>(self.blocks.len().min(u32::MAX as usize) as u32)?;
        writer.write_u8(if self.has_transport { 1 } else { 0 })?;

        for block in &self.blocks {
            <UpdateData as OrderedWrite<T>>::write(block, writer)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum UpdateData {
    UpdateObject {
        guid: AnyGuid,
        values: ValuesUpdate,
    },
    Movement {},
    // Difference between CreateObject and CreateNewObject is unknown
    CreateObject {
        guid: AnyGuid,
        movement: MovementUpdate,
        values: ValuesUpdate,
    },
    CreateNewObject {
        guid: AnyGuid,
        movement: MovementUpdate,
        values: ValuesUpdate,
    },
    OutOfRangeDestroyObject {
        guids: Vec<AnyGuid>,
    },
    ForceDestroyObject {
        guids: Vec<AnyGuid>,
    },
}

impl<T: ByteOrder> OrderedWrite<T> for UpdateData {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        match self {
            UpdateData::UpdateObject { guid, values } => {
                writer.write_u8(0)?; //PARTIAL UpdateType
                write_packed_guid(guid, writer)?;
                <ValuesUpdate as OrderedWrite<LittleEndian>>::write(values, writer)?;
            }
            UpdateData::Movement {} => todo!(),
            UpdateData::CreateObject {
                guid,
                movement,
                values,
            } => {
                writer.write_u8(2)?; //CREATE_OBJECT UpdateType
                write_packed_guid(guid, writer)?;
                let type_id = match guid {
                    AnyGuid::Item(_) => 1,
                    AnyGuid::Container(_) => 2,
                    AnyGuid::Unit(_) => 3,
                    AnyGuid::Player(_) => 4,
                    AnyGuid::GameObject(_) => 5,
                    AnyGuid::DynamicObject(_) => 6,
                    AnyGuid::Corpse(_) => 7,
                };
                writer.write_u8(type_id)?;
                <MovementUpdate as OrderedWrite<LittleEndian>>::write(movement, writer)?;
                <ValuesUpdate as OrderedWrite<LittleEndian>>::write(values, writer)?;
            }
            UpdateData::CreateNewObject {
                guid,
                movement,
                values,
            } => {
                writer.write_u8(3)?; //CREATE_OBJECT2 UpdateType
                write_packed_guid(guid, writer)?;
                let type_id = match guid {
                    AnyGuid::Item(_) => 1,
                    AnyGuid::Container(_) => 2,
                    AnyGuid::Unit(_) => 3,
                    AnyGuid::Player(_) => 4,
                    AnyGuid::GameObject(_) => 5,
                    AnyGuid::DynamicObject(_) => 6,
                    AnyGuid::Corpse(_) => 7,
                };
                writer.write_u8(type_id)?;
                <MovementUpdate as OrderedWrite<LittleEndian>>::write(movement, writer)?;
                <ValuesUpdate as OrderedWrite<LittleEndian>>::write(values, writer)?;
            }
            UpdateData::OutOfRangeDestroyObject { guids } => {
                writer.write_u8(4)?; //FAR_OBJECTS UpdateType
                debug_assert!(guids.len() <= u32::MAX as usize); //Should never happen in any realistic scenario
                writer.write_u32::<LittleEndian>(guids.len().min(u32::MAX as usize) as u32)?;
                for guid in guids {
                    write_packed_guid(guid, writer)?;
                }
            }
            UpdateData::ForceDestroyObject { guids } => {
                writer.write_u8(5)?; //NEAR_OBJECTS UpdateType
                debug_assert!(guids.len() <= u32::MAX as usize); //Should never happen in any realistic scenario
                writer.write_u32::<LittleEndian>(guids.len().min(u32::MAX as usize) as u32)?;
                for guid in guids {
                    write_packed_guid(guid, writer)?;
                }
            }
        }

        Ok(())
    }
}

fn write_packed_guid(guid: &AnyGuid, writer: &mut Vec<u8>) -> Result<(), std::io::Error> {
    let v = guid.get().get();

    let mut bitvec = BitVec::<u8>::default();
    bitvec.extend(std::iter::repeat(false).take(8));
    for (i, b) in v.to_le_bytes().into_iter().enumerate() {
        if b != 0 {
            bitvec.set(i, true);
        }
    }
    writer.write_u8(bitvec.blocks().next().unwrap())?;
    for b in v.to_le_bytes() {
        if b != 0 {
            writer.write_u8(b)?;
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum PossibleUpdate<T> {
    Value(T),
    Clear,
    NoUpdate,
}

#[derive(Debug)]
pub struct MovementUpdate {
    pub is_self_update: bool,
    pub position: Option<PositionUpdate>,
    pub high_guid: Option<u32>,
    pub is_update_all: bool,
    pub full_guid: PossibleUpdate<AnyGuid>,
    pub transport_time_millis: Option<u32>,
}

#[bitfield(u8)]
pub struct UpdateFlags {
    pub updates_self: bool,
    pub transport: bool,
    pub full_guid: bool,
    pub high_guid: bool,
    pub all: bool,
    pub living: bool,
    pub has_position: bool,
    pub unkn: bool,
}

impl MovementUpdate {
    pub fn get_update_flags(&self) -> UpdateFlags {
        UpdateFlags::new()
            .with_updates_self(self.is_self_update)
            .with_transport(self.transport_time_millis.is_some())
            .with_full_guid(!matches!(self.full_guid, PossibleUpdate::NoUpdate))
            .with_high_guid(self.high_guid.is_some())
            .with_all(self.is_update_all)
            .with_living(
                self.position
                    .as_ref()
                    .is_some_and(|v| matches!(v, PositionUpdate::Living { .. })),
            )
            .with_has_position(
                self.position
                    .as_ref()
                    .is_some_and(|v| matches!(v, PositionUpdate::NonLiving { .. })),
            )
    }
}

impl<T: ByteOrder> OrderedWrite<T> for MovementUpdate {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        let update_flags = self.get_update_flags();
        writer.write_u8(update_flags.0)?;

        if let Some(position) = &self.position {
            match position {
                PositionUpdate::Living {
                    movement_info,
                    walk_speed,
                    run_speed,
                    run_backwards_speed,
                    swim_speed,
                    swim_backwards_speed,
                    turn_speed,
                } => {
                    <MovementInfo as OrderedWrite<LittleEndian>>::write(movement_info, writer)?;
                    writer.write_f32::<LittleEndian>(*walk_speed)?;
                    writer.write_f32::<LittleEndian>(*run_speed)?;
                    writer.write_f32::<LittleEndian>(*run_backwards_speed)?;
                    writer.write_f32::<LittleEndian>(*swim_speed)?;
                    writer.write_f32::<LittleEndian>(*swim_backwards_speed)?;
                    writer.write_f32::<LittleEndian>(*turn_speed)?;
                    //TODO: spline
                }
                PositionUpdate::NonLiving {
                    pos_x,
                    pos_y,
                    pos_z,
                    orientation,
                } => {
                    writer.write_f32::<LittleEndian>(*pos_x)?;
                    writer.write_f32::<LittleEndian>(*pos_y)?;
                    writer.write_f32::<LittleEndian>(*pos_z)?;
                    writer.write_f32::<LittleEndian>(*orientation)?;
                }
            }
        }

        if let Some(v) = self.high_guid {
            writer.write_u32::<LittleEndian>(v)?;
        }
        if self.is_update_all {
            writer.write_u32::<LittleEndian>(1)?;
        }
        match &self.full_guid {
            PossibleUpdate::Value(v) => {
                write_packed_guid(v, writer)?;
            }
            PossibleUpdate::Clear => {
                writer.write_u8(0)?;
            }
            PossibleUpdate::NoUpdate => (),
        }
        if let Some(v) = self.transport_time_millis {
            writer.write_u32::<LittleEndian>(v)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum PositionUpdate {
    NonLiving {
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
        orientation: f32,
    },
    Living {
        movement_info: MovementInfo,
        walk_speed: f32,
        run_speed: f32,
        run_backwards_speed: f32,
        swim_speed: f32,
        swim_backwards_speed: f32,
        turn_speed: f32,
    },
}

#[derive(Debug)]
pub struct ValuesUpdate {
    pub mask_blocks: BitVec<u32>,
    pub values_blocks: Vec<u32>,
}

impl<T: ByteOrder> OrderedWrite<T> for ValuesUpdate {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        let blocks = {
            let storage = self.mask_blocks.storage();
            let mut l = storage.len();
            for v in storage.iter().rev() {
                if *v != 0 {
                    break;
                }
                l -= 1;
            }
            &storage[0..l]
        };
        writer.write_u8(blocks.len() as u8)?;
        for block in blocks {
            writer.write_u32::<LittleEndian>(*block)?;
        }
        for block in &self.values_blocks {
            writer.write_u32::<LittleEndian>(*block)?;
        }

        Ok(())
    }
}

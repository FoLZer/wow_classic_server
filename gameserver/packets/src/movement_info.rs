use std::io::{Cursor, ErrorKind};

use bitfield_struct::bitfield;
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use common::guid::{self, Guid};

use crate::{client::OrderedRead, server::OrderedWrite};

#[derive(Debug, Clone)]
pub struct MovementInfo {
    pub movement_flags: MovementFlags,
    pub timestamp: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub orientation: f32,
    pub on_transport_data: Option<MovementInfoTransportData>,
    pub swimming_pitch: Option<f32>,
    pub fall_time: Option<u32>,
    pub falling_data: Option<MovementInfoFallingData>,
    pub spline_elevation: Option<f32>,
}

impl<T: ByteOrder> OrderedWrite<T> for MovementInfo {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u32::<LittleEndian>(self.movement_flags.into())?;
        writer.write_u32::<LittleEndian>(self.timestamp)?;
        writer.write_f32::<LittleEndian>(self.pos_x)?;
        writer.write_f32::<LittleEndian>(self.pos_y)?;
        writer.write_f32::<LittleEndian>(self.pos_z)?;
        writer.write_f32::<LittleEndian>(self.orientation)?;
        if self.movement_flags.on_transport() {
            let data = self.on_transport_data.as_ref().unwrap(); //TODO: check
            writer.write_u64::<LittleEndian>(data.guid.get().get())?;
            writer.write_f32::<LittleEndian>(data.pos_x)?;
            writer.write_f32::<LittleEndian>(data.pos_y)?;
            writer.write_f32::<LittleEndian>(data.pos_z)?;
            writer.write_f32::<LittleEndian>(data.orientation)?;
            writer.write_u32::<LittleEndian>(data.time)?;
        }
        if self.movement_flags.swimming() {
            let data = self.swimming_pitch.unwrap(); //TODO: check
            writer.write_f32::<LittleEndian>(data)?;
        }
        if !self.movement_flags.on_transport() {
            let data = self.fall_time.unwrap(); //TODO: check
            writer.write_u32::<LittleEndian>(data)?;
        }
        if self.movement_flags.falling() {
            let data = self.falling_data.as_ref().unwrap(); //TODO: check
            writer.write_f32::<LittleEndian>(data.velocity)?;
            writer.write_f32::<LittleEndian>(data.sin_angle)?;
            writer.write_f32::<LittleEndian>(data.cos_angle)?;
            writer.write_f32::<LittleEndian>(data.xy_speed)?;
        }
        if self.movement_flags.spline_elevation() {
            let data = self.spline_elevation.unwrap(); //TODO: check
            writer.write_f32::<LittleEndian>(data)?;
        }

        Ok(())
    }
}

#[bitfield(u32)]
pub struct MovementFlags {
    pub forward: bool,
    pub backward: bool,
    pub strafe_left: bool,
    pub strafe_right: bool,
    pub turn_left: bool,
    pub turn_right: bool,
    pub pitch_up: bool,
    pub pitch_down: bool,
    pub walk_mode: bool,
    _unkn_1: bool,
    pub levitating: bool,
    pub flying: bool,
    _unkn_2: bool,
    pub falling: bool,
    pub falling_far: bool,
    _unkn_3: bool,
    _unkn_4: bool,
    _unkn_5: bool,
    _unkn_6: bool,
    _unkn_7: bool,
    _unkn_8: bool,
    pub swimming: bool,
    pub spline_enabled: bool,
    pub can_fly: bool,
    pub flying_old: bool,
    pub on_transport: bool,
    pub spline_elevation: bool,
    pub root: bool,
    pub water_walking: bool,
    pub safe_fall: bool,
    pub hover: bool,
    _unkn_9: bool,
}

#[derive(Debug, Clone)]
pub struct MovementInfoTransportData {
    pub guid: Guid<guid::DynamicObject>,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub orientation: f32,
    pub time: u32,
}

#[derive(Debug, Clone)]
pub struct MovementInfoFallingData {
    pub velocity: f32,
    pub sin_angle: f32,
    pub cos_angle: f32,
    pub xy_speed: f32,
}

impl<T: ByteOrder> OrderedRead<T> for MovementInfo {
    fn from_reader(reader: &mut Cursor<&[u8]>) -> std::io::Result<Self> {
        let movement_flags = MovementFlags::from_bits(reader.read_u32::<LittleEndian>()?);
        let timestamp = reader.read_u32::<LittleEndian>()?;
        let pos_x = reader.read_f32::<LittleEndian>()?;
        let pos_y = reader.read_f32::<LittleEndian>()?;
        let pos_z = reader.read_f32::<LittleEndian>()?;
        let orientation = reader.read_f32::<LittleEndian>()?;

        let on_transport_data = if movement_flags.on_transport() {
            let guid = Guid::try_from_u64(reader.read_u64::<LittleEndian>()?).ok_or(
                std::io::Error::new(ErrorKind::InvalidData, "wrong guid received"),
            )?;
            let pos_x = reader.read_f32::<LittleEndian>()?;
            let pos_y = reader.read_f32::<LittleEndian>()?;
            let pos_z = reader.read_f32::<LittleEndian>()?;
            let orientation = reader.read_f32::<LittleEndian>()?;
            let time = reader.read_u32::<LittleEndian>()?;

            Some(MovementInfoTransportData {
                guid,
                pos_x,
                pos_y,
                pos_z,
                orientation,
                time,
            })
        } else {
            None
        };

        let swimming_pitch = if movement_flags.swimming() {
            Some(reader.read_f32::<LittleEndian>()?)
        } else {
            None
        };

        let fall_time = if movement_flags.on_transport() {
            Some(reader.read_u32::<LittleEndian>()?)
        } else {
            None
        };

        let falling_data = if movement_flags.falling() {
            let velocity = reader.read_f32::<LittleEndian>()?;
            let sin_angle = reader.read_f32::<LittleEndian>()?;
            let cos_angle = reader.read_f32::<LittleEndian>()?;
            let xy_speed = reader.read_f32::<LittleEndian>()?;

            Some(MovementInfoFallingData {
                velocity,
                sin_angle,
                cos_angle,
                xy_speed,
            })
        } else {
            None
        };

        let spline_elevation = if movement_flags.spline_elevation() {
            Some(reader.read_f32::<LittleEndian>()?)
        } else {
            None
        };

        Ok(Self {
            movement_flags,
            timestamp,
            pos_x,
            pos_y,
            pos_z,
            orientation,
            on_transport_data,
            swimming_pitch,
            fall_time,
            falling_data,
            spline_elevation,
        })
    }
}

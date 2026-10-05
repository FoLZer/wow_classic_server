use bitfield_struct::bitfield;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};

use crate::server::OrderedWrite;

#[bitfield(u32)]
pub struct HitInfo {
    _extended_data: bool, // makes the client read an extra damage calculation block, never sent
    pub affects_victim: bool, // victim plays the wound animation
    pub off_hand: bool,   // attacker swings with the off hand
    _unkn: bool,
    pub miss: bool,
    pub absorb: bool,   // shows absorb floating text when no damage was dealt
    pub resist: bool,   // shows resist floating text when no damage was dealt
    pub critical: bool, // critical floating text and wound animation
    #[bits(4)]
    _unkn: u8,
    pub hide_miss_text: bool, // suppresses miss/absorb/resist floating text
    pub alternate_impact_effect: bool, // picks the alternate impact effect on the victim
    _unkn: bool,              // never read by the client
    pub crushing: bool,       // crushing wound animation, takes precedence over critical
    pub no_swing_animation: bool,
    #[bits(2)]
    _unkn: u8,
    pub no_impact_sound: bool,
    #[bits(12)]
    _unkn: u16,
}

impl<T: ByteOrder> OrderedWrite<T> for HitInfo {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        // the extended data block isn't supported, make sure the client never expects it
        writer.write_u32::<LittleEndian>(self.0 & !1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VictimState {
    Unaffected = 0,
    Normal = 1,
    Dodge = 2,
    Parry = 3,
    Interrupt = 4,
    Block = 5, // also forced by the client when total_damage is 0 and blocked is not
    Evade = 6,
    Immune = 7,
    Deflect = 8,
}

impl<T: ByteOrder> OrderedWrite<T> for VictimState {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u32::<LittleEndian>(*self as u32)
    }
}

// Resistances.dbc
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageSchool {
    Physical = 0,
    Holy = 1,
    Fire = 2,
    Nature = 3,
    Frost = 4,
    Shadow = 5,
    Arcane = 6,
}

impl<T: ByteOrder> OrderedWrite<T> for DamageSchool {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_u32::<LittleEndian>(*self as u32)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SubDamage {
    pub school: DamageSchool,
    pub damage: f32,
    pub int_damage: u32,
    pub absorbed: u32,
    pub resisted: u32,
}

impl<T: ByteOrder> OrderedWrite<T> for SubDamage {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        <DamageSchool as OrderedWrite<LittleEndian>>::write(&self.school, writer)?;
        <f32 as OrderedWrite<LittleEndian>>::write(&self.damage, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.int_damage, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.absorbed, writer)?;
        <u32 as OrderedWrite<LittleEndian>>::write(&self.resisted, writer)?;

        Ok(())
    }
}

// SubDamages can only have MAX_LEN entries at max, the server will crash if this is not accounted for
pub struct SubDamages(pub Vec<SubDamage>);

impl SubDamages {
    // the client stores sub damages in fixed arrays without checking the count
    pub const MAX_LEN: usize = 5;
}

impl<T: ByteOrder> OrderedWrite<T> for SubDamages {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        assert!(
            self.0.len() <= Self::MAX_LEN,
            "SubDamages must not have more than 5 entries to be sent to the client"
        );

        <Vec<SubDamage> as OrderedWrite<LittleEndian>>::write(&self.0, writer)
    }
}

pub enum HitHand {
    Main,
    Off
}

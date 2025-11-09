use bincode::{Decode, Encode};

#[derive(Decode, Encode, Clone)]
pub enum RealmType {
    Normal,
    PvP,
    RP,
    RPPvP,
}

impl From<RealmType> for u32 {
    fn from(val: RealmType) -> Self {
        match val {
            RealmType::Normal => 0,
            RealmType::PvP => 1,
            RealmType::RP => 6,
            RealmType::RPPvP => 8,
        }
    }
}

impl TryFrom<u8> for RealmType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(RealmType::Normal),
            1 => Ok(RealmType::PvP),
            6 => Ok(RealmType::RP),
            8 => Ok(RealmType::RPPvP),
            _ => Err(()),
        }
    }
}

#[derive(Decode, Encode, Clone)]
pub enum RealmCategory {
    Unkn,
}

impl From<RealmCategory> for u8 {
    fn from(val: RealmCategory) -> Self {
        match val {
            RealmCategory::Unkn => 1,
        }
    }
}

impl TryFrom<u8> for RealmCategory {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(RealmCategory::Unkn),
            _ => Err(()),
        }
    }
}

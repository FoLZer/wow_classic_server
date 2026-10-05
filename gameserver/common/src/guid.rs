use std::{
    marker::PhantomData,
    num::{NonZeroU32, NonZeroU64},
};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Guid<T: GuidType>(NonZeroU32, PhantomData<T>);

impl<T: GuidType> Guid<T> {
    pub fn from_u32(v: NonZeroU32) -> Self {
        Self(v, PhantomData)
    }

    pub fn try_from_u64(v: u64) -> Option<Self> {
        let prefix = (v >> (32 + 16)) as u16;
        let value = (v & 0x0000_0000_FFFF_FFFF) as u32;
        if T::PREFIX != prefix {
            return None;
        }

        Some(Self(NonZeroU32::new(value)?, PhantomData))
    }

    pub fn get(&self) -> NonZeroU64 {
        NonZeroU64::from(self.0) | ((T::PREFIX as u64) << (32 + 16))
    }

    pub fn get_u32(&self) -> NonZeroU32 {
        self.0
    }
}

pub trait GuidType {
    const PREFIX: u16;
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Item;

impl GuidType for Item {
    const PREFIX: u16 = 0x4000;
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Container;

impl GuidType for Container {
    const PREFIX: u16 = 0x4000;
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Unit;

impl GuidType for Unit {
    const PREFIX: u16 = 0xF130;
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Player;

impl GuidType for Player {
    const PREFIX: u16 = 0x0000;
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct GameObject;

impl GuidType for GameObject {
    const PREFIX: u16 = 0xF110;
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct DynamicObject;

impl GuidType for DynamicObject {
    const PREFIX: u16 = 0xF100;
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Corpse;

impl GuidType for Corpse {
    const PREFIX: u16 = 0xF101;
}

#[derive(Debug, Clone, Copy)]
pub enum AnyGuid {
    Item(Guid<Item>),
    Container(Guid<Container>),
    Unit(Guid<Unit>),
    Player(Guid<Player>),
    GameObject(Guid<GameObject>),
    DynamicObject(Guid<DynamicObject>),
    Corpse(Guid<Corpse>),
}

impl AnyGuid {
    pub fn get(&self) -> NonZeroU64 {
        match self {
            AnyGuid::Item(guid) => guid.get(),
            AnyGuid::Container(guid) => guid.get(),
            AnyGuid::Unit(guid) => guid.get(),
            AnyGuid::Player(guid) => guid.get(),
            AnyGuid::GameObject(guid) => guid.get(),
            AnyGuid::DynamicObject(guid) => guid.get(),
            AnyGuid::Corpse(guid) => guid.get(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectableGuid {
    Unit(Guid<Unit>),
    Player(Guid<Player>),
    Corpse(Guid<Corpse>),
}

impl SelectableGuid {
    pub fn try_from_u64(v: u64) -> Option<Self> {
        let prefix = (v >> 48) as u16;

        match prefix {
            Unit::PREFIX => Guid::<Unit>::try_from_u64(v).map(Self::Unit),
            Player::PREFIX => Guid::<Player>::try_from_u64(v).map(Self::Player),
            Corpse::PREFIX => Guid::<Corpse>::try_from_u64(v).map(Self::Corpse),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivingGuid {
    Unit(Guid<Unit>),
    Player(Guid<Player>),
}

impl LivingGuid {
    pub fn get(&self) -> NonZeroU64 {
        match self {
            LivingGuid::Unit(guid) => guid.get(),
            LivingGuid::Player(guid) => guid.get(),
        }
    }

    pub fn try_from_u64(v: u64) -> Option<Self> {
        let prefix = (v >> 48) as u16;

        match prefix {
            Unit::PREFIX => Guid::<Unit>::try_from_u64(v).map(Self::Unit),
            Player::PREFIX => Guid::<Player>::try_from_u64(v).map(Self::Player),
            _ => None,
        }
    }
}

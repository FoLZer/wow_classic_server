use std::{
    marker::PhantomData,
    num::{NonZeroU32, NonZeroU64},
};

#[derive(Clone, Copy)]
pub struct Guid<T: GuidType>(NonZeroU32, PhantomData<T>);

impl<T: GuidType> Guid<T> {
    pub fn from_u32(v: NonZeroU32) -> Self {
        Self(v, PhantomData)
    }

    pub fn try_from_u64(v: u64) -> Option<Self> {
        let prefix = (v >> (32 + 16)) as u16;
        let value = (v & 0x0000_0000_FFFF_FFFF) as u32;
        if T::get_prefix() != prefix {
            return None;
        }

        Some(Self(NonZeroU32::new(value)?, PhantomData))
    }

    pub fn get(&self) -> NonZeroU64 {
        NonZeroU64::from(self.0) | ((T::get_prefix() as u64) << (32 + 16))
    }

    pub fn get_u32(&self) -> NonZeroU32 {
        self.0
    }
}

pub trait GuidType {
    fn get_prefix() -> u16;
}

#[derive(Clone)]
pub struct Player;

impl GuidType for Player {
    fn get_prefix() -> u16 {
        0x0000
    }
}

#[derive(Clone)]
pub struct Unit;

impl GuidType for Unit {
    fn get_prefix() -> u16 {
        0xF130
    }
}

#[derive(Clone)]
pub struct GameObject;

impl GuidType for GameObject {
    fn get_prefix() -> u16 {
        0xF110
    }
}

#[derive(Clone)]
pub struct DynamicObject;

impl GuidType for DynamicObject {
    fn get_prefix() -> u16 {
        0xF100
    }
}

use std::{
    ffi::{CString, NulError},
    fmt,
    io::{ErrorKind, Write},
    ops::Deref,
};

use byteorder::{ByteOrder, WriteBytesExt};

use crate::{gossip::U32Counted, server::OrderedWrite};

#[derive(Debug)]
pub struct LengthError {
    pub len: usize,
    pub max: usize,
}

impl fmt::Display for LengthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "length {} exceeds the limit of {}", self.len, self.max)
    }
}

impl std::error::Error for LengthError {}

impl From<LengthError> for std::io::Error {
    fn from(value: LengthError) -> Self {
        std::io::Error::new(ErrorKind::InvalidData, value)
    }
}

#[derive(Debug)]
pub enum BoundedStringError {
    Nul(NulError),
    Length(LengthError),
}

impl fmt::Display for BoundedStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nul(e) => e.fmt(f),
            Self::Length(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for BoundedStringError {}

pub trait BoundedLen {
    const EMPTY_LEN: usize;

    fn bounded_len(&self) -> usize;
}

impl BoundedLen for CString {
    const EMPTY_LEN: usize = 1;

    fn bounded_len(&self) -> usize {
        self.as_bytes_with_nul().len()
    }
}

impl<T> BoundedLen for Vec<T> {
    const EMPTY_LEN: usize = 0;

    fn bounded_len(&self) -> usize {
        self.len()
    }
}

impl<T> BoundedLen for U32Counted<T> {
    const EMPTY_LEN: usize = 0;

    fn bounded_len(&self) -> usize {
        self.0.len()
    }
}

#[derive(Debug, Clone)]
pub struct Bounded<T, const MAX: usize>(T);

impl<T: BoundedLen, const MAX_LENGTH: usize> Bounded<T, MAX_LENGTH> {
    pub const MAX_LENGTH: usize = MAX_LENGTH;

    pub fn new(value: T) -> Result<Self, LengthError> {
        let len = value.bounded_len();
        if len > MAX_LENGTH {
            return Err(LengthError { len, max: MAX_LENGTH });
        }

        Ok(Self(value))
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: BoundedLen + Default, const MAX: usize> Default for Bounded<T, MAX> {
    fn default() -> Self {
        const { assert!(T::EMPTY_LEN <= MAX, "the empty value exceeds the limit") };
        Self(T::default())
    }
}

impl<const MAX: usize> TryFrom<&str> for Bounded<CString, MAX> {
    type Error = BoundedStringError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let value = CString::new(value).map_err(BoundedStringError::Nul)?;
        Self::new(value).map_err(BoundedStringError::Length)
    }
}

impl<T, const MAX: usize> Bounded<Vec<T>, MAX> {
    pub fn from_array<const N: usize>(value: [T; N]) -> Self {
        const { assert!(N <= MAX, "too many entries") };
        Self(value.into())
    }
}

impl<T, const MAX: usize> Deref for Bounded<T, MAX> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T, const MAX: usize> IntoIterator for &'a Bounded<T, MAX>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.0).into_iter()
    }
}

impl<T: ByteOrder, const MAX: usize> OrderedWrite<T> for Bounded<CString, MAX> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        writer.write_all(self.0.as_bytes_with_nul())
    }

    fn serialized_size(&self) -> usize {
        self.0.as_bytes_with_nul().len()
    }
}

impl<T: ByteOrder, const MAX: usize> OrderedWrite<T> for Option<Bounded<CString, MAX>> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        match self {
            Some(v) => <Bounded<CString, MAX> as OrderedWrite<T>>::write(v, writer),
            None => writer.write_u8(0),
        }
    }

    fn serialized_size(&self) -> usize {
        self.as_ref()
            .map_or(1, |value| value.0.as_bytes_with_nul().len())
    }
}

impl<B: ByteOrder, T: OrderedWrite<B>, const MAX: usize> OrderedWrite<B> for Bounded<Vec<T>, MAX> {
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        const { assert!(MAX <= u8::MAX as usize, "the count must fit a u8") };
        writer.write_u8(self.0.len() as u8)?;
        for v in &self.0 {
            v.write(writer)?;
        }
        Ok(())
    }

    fn serialized_size(&self) -> usize {
        size_of::<u8>()
            + self
                .0
                .iter()
                .map(OrderedWrite::<B>::serialized_size)
                .sum::<usize>()
    }
}

impl<B: ByteOrder, T: OrderedWrite<B>, const MAX: usize> OrderedWrite<B>
    for Bounded<U32Counted<T>, MAX>
{
    fn write(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        const { assert!(MAX <= u32::MAX as usize, "the count must fit a u32") };
        <U32Counted<T> as OrderedWrite<B>>::write(&self.0, writer)
    }

    fn serialized_size(&self) -> usize {
        size_of::<u32>()
            + self
                .0
                .0
                .iter()
                .map(OrderedWrite::<B>::serialized_size)
                .sum::<usize>()
    }
}

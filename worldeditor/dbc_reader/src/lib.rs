#![feature(fn_traits)]
#![feature(iterator_try_collect)]
#![feature(cursor_split)]

pub mod structs;

use byteorder::{LittleEndian, ReadBytesExt};
use std::{ffi::CString, io::Read, sync::Arc};
use structs::{DB2File, DBCFile, LocString, Record, StringIndex, WDBFile};

pub fn read_dbc<R: Read, Rec: Record + 'static + Send + Sync>(
    reader: &mut R,
) -> Result<Arc<dyn WDBFile<Rec> + Send + Sync>, std::io::Error> {
    let magic = {
        let mut v = [0; 4];
        reader.read_exact(&mut v)?;
        v
    };

    match &magic {
        b"WDBC" => Ok(Arc::new(DBCFile::<Rec>::from_reader(reader)?)),
        b"WDB2" => Ok(Arc::new(DB2File::<Rec>::from_reader(reader)?)),
        b"WDB3" => {
            todo!()
        }
        b"WDB4" => {
            todo!()
        }
        b"WDB5" => {
            todo!()
        }
        b"WDB6" => {
            todo!()
        }
        b"WDC1" => {
            todo!()
        }
        b"WDC2" | b"1SLC" => {
            todo!()
        }
        b"WDC3" => {
            todo!()
        }
        b"WDC4" => {
            todo!()
        }
        _ => Err(std::io::Error::from(std::io::ErrorKind::InvalidData)),
    }

    //Ok(())
}

impl Record for i8 {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        reader.read_i8()
    }
}

impl Record for u8 {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        reader.read_u8()
    }
}

impl Record for i16 {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        reader.read_i16::<LittleEndian>()
    }
}

impl Record for u16 {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        reader.read_u16::<LittleEndian>()
    }
}

impl Record for i32 {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        reader.read_i32::<LittleEndian>()
    }
}

impl Record for u32 {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        reader.read_u32::<LittleEndian>()
    }
}

impl Record for i64 {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        reader.read_i64::<LittleEndian>()
    }
}

impl Record for u64 {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        reader.read_u64::<LittleEndian>()
    }
}

impl Record for f32 {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        reader.read_f32::<LittleEndian>()
    }
}

impl Record for f64 {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        reader.read_f64::<LittleEndian>()
    }
}

impl Record for StringIndex {
    fn from_reader<R: Read>(
        reader: &mut R,
        _cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(StringIndex(reader.read_u32::<LittleEndian>()?))
    }
}

impl Record for CString {
    fn from_reader<R: Read>(
        reader: &mut R,
        cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        cstring_reader.call((reader,))
    }
}

impl<const S: usize> Record for LocString<S> {
    fn from_reader<R: Read>(
        reader: &mut R,
        cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(LocString {
            locales: {
                {
                    //let mut v = [StringIndex(0); S];
                    //for d in &mut v {
                    //    d.0 = reader.read_u32::<LittleEndian>()?;
                    //}

                    std::iter::repeat_with(|| cstring_reader(reader))
                        .take(S)
                        .try_collect::<Vec<CString>>()?
                        .try_into()
                        .unwrap()
                }
            },
            flags: reader.read_u32::<LittleEndian>()?,
        })
    }
}

impl<Rec: Record, const S: usize> Record for [Rec; S] {
    fn from_reader<R: Read>(
        reader: &mut R,
        cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        let mut v = Vec::with_capacity(S);
        for _ in 0..S {
            v.push(Rec::from_reader(reader, cstring_reader)?);
        }
        let Ok(ar) = v.try_into() else { panic!() };
        Ok(ar)
    }
}

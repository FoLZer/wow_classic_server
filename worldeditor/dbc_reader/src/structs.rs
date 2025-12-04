use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    io::{Cursor, Read},
    sync::Arc,
};

use byteorder::{LittleEndian, ReadBytesExt};

pub trait Record {
    fn from_reader<R: Read>(
        reader: &mut R,
        cstring_reader: &impl Fn(&mut R) -> Result<CString, std::io::Error>,
    ) -> Result<Self, std::io::Error>
    where
        Self: Sized;
}

pub trait WDBFile<Rec> {
    fn get_records(&self) -> Arc<[Rec]>;
}

#[derive(Debug)]
pub struct DBCHeader {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_table_size: u32,
}
impl DBCHeader {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            record_count: reader.read_u32::<LittleEndian>()?,
            field_count: reader.read_u32::<LittleEndian>()?,
            record_size: reader.read_u32::<LittleEndian>()?,
            string_table_size: reader.read_u32::<LittleEndian>()?,
        })
    }
}
pub struct DB2Header {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_table_size: u32,
    pub table_hash: u32,
    pub build: u32,
    pub timestamp_last_written: u32,
    pub min_id: u32,
    pub max_id: u32,
    pub locale: Locale,
    pub copy_table_size: u32,
}
impl DB2Header {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            record_count: reader.read_u32::<LittleEndian>()?,
            field_count: reader.read_u32::<LittleEndian>()?,
            record_size: reader.read_u32::<LittleEndian>()?,
            string_table_size: reader.read_u32::<LittleEndian>()?,
            table_hash: reader.read_u32::<LittleEndian>()?,
            build: reader.read_u32::<LittleEndian>()?,
            timestamp_last_written: reader.read_u32::<LittleEndian>()?,
            min_id: reader.read_u32::<LittleEndian>()?,
            max_id: reader.read_u32::<LittleEndian>()?,
            locale: Locale::try_from(reader.read_u32::<LittleEndian>()?)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?,
            copy_table_size: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[allow(non_camel_case_types)]
pub enum Locale {
    enUS = 0,
    koKR = 1,
    frFR = 2,
    deDE = 3,
    enCN = 4,
    enTW = 5,
    esES = 6,
    esMX = 7,
    ruRU = 8,
    ptPT = 10,
    itIT = 11,
}
impl TryFrom<u32> for Locale {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Locale::enUS),
            1 => Ok(Locale::koKR),
            2 => Ok(Locale::frFR),
            3 => Ok(Locale::deDE),
            4 => Ok(Locale::enCN),
            5 => Ok(Locale::enTW),
            6 => Ok(Locale::esES),
            7 => Ok(Locale::esMX),
            8 => Ok(Locale::ruRU),
            10 => Ok(Locale::ptPT),
            11 => Ok(Locale::itIT),
            _ => Err(()),
        }
    }
}

pub struct DB4Header {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_table_size: u32,
    pub table_hash: u32,
    pub build: u32,
    pub timestamp_last_written: u32,
    pub min_id: u32,
    pub max_id: u32,
    pub locale: Locale,
    pub copy_table_size: u32,
    pub flags: u32,
}
impl DB4Header {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            record_count: reader.read_u32::<LittleEndian>()?,
            field_count: reader.read_u32::<LittleEndian>()?,
            record_size: reader.read_u32::<LittleEndian>()?,
            string_table_size: reader.read_u32::<LittleEndian>()?,
            table_hash: reader.read_u32::<LittleEndian>()?,
            build: reader.read_u32::<LittleEndian>()?,
            timestamp_last_written: reader.read_u32::<LittleEndian>()?,
            min_id: reader.read_u32::<LittleEndian>()?,
            max_id: reader.read_u32::<LittleEndian>()?,
            locale: Locale::try_from(reader.read_u32::<LittleEndian>()?)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?,
            copy_table_size: reader.read_u32::<LittleEndian>()?,
            flags: reader.read_u32::<LittleEndian>()?,
        })
    }
}
pub struct DB5Header {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_table_size: u32,
    pub table_hash: u32,
    pub layout_hash: u32,
    pub min_id: u32,
    pub max_id: u32,
    pub locale: Locale,
    pub copy_table_size: u32,
    pub flags: u16,
    pub id_index: u16,
}
impl DB5Header {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            record_count: reader.read_u32::<LittleEndian>()?,
            field_count: reader.read_u32::<LittleEndian>()?,
            record_size: reader.read_u32::<LittleEndian>()?,
            string_table_size: reader.read_u32::<LittleEndian>()?,
            table_hash: reader.read_u32::<LittleEndian>()?,
            layout_hash: reader.read_u32::<LittleEndian>()?,
            min_id: reader.read_u32::<LittleEndian>()?,
            max_id: reader.read_u32::<LittleEndian>()?,
            locale: Locale::try_from(reader.read_u32::<LittleEndian>()?)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?,
            copy_table_size: reader.read_u32::<LittleEndian>()?,
            flags: reader.read_u16::<LittleEndian>()?,
            id_index: reader.read_u16::<LittleEndian>()?,
        })
    }
}
pub struct DB6Header {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_table_size: u32,
    pub table_hash: u32,
    pub layout_hash: u32,
    pub min_id: u32,
    pub max_id: u32,
    pub locale: Locale,
    pub copy_table_size: u32,
    pub flags: u16,
    pub id_index: u16,
    pub total_field_count: u32,
    pub common_data_table_size: u32,
}
impl DB6Header {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            record_count: reader.read_u32::<LittleEndian>()?,
            field_count: reader.read_u32::<LittleEndian>()?,
            record_size: reader.read_u32::<LittleEndian>()?,
            string_table_size: reader.read_u32::<LittleEndian>()?,
            table_hash: reader.read_u32::<LittleEndian>()?,
            layout_hash: reader.read_u32::<LittleEndian>()?,
            min_id: reader.read_u32::<LittleEndian>()?,
            max_id: reader.read_u32::<LittleEndian>()?,
            locale: Locale::try_from(reader.read_u32::<LittleEndian>()?)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?,
            copy_table_size: reader.read_u32::<LittleEndian>()?,
            flags: reader.read_u16::<LittleEndian>()?,
            id_index: reader.read_u16::<LittleEndian>()?,
            total_field_count: reader.read_u32::<LittleEndian>()?,
            common_data_table_size: reader.read_u32::<LittleEndian>()?,
        })
    }
}
pub struct DC1Header {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_table_size: u32,
    pub table_hash: u32,
    pub layout_hash: u32,
    pub min_id: u32,
    pub max_id: u32,
    pub locale: Locale,
    pub copy_table_size: u32,
    pub flags: u16,
    pub id_index: u16,
    pub total_field_count: u32,
    pub bitpacked_data_offset: u32,
    pub lookup_column_count: u32,
    pub offset_map_offset: u32,
    pub id_list_size: u32,
    pub field_storage_info_size: u32,
    pub common_data_size: u32,
    pub pallet_data_size: u32,
    pub relationship_data_size: u32,
}
impl DC1Header {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            record_count: reader.read_u32::<LittleEndian>()?,
            field_count: reader.read_u32::<LittleEndian>()?,
            record_size: reader.read_u32::<LittleEndian>()?,
            string_table_size: reader.read_u32::<LittleEndian>()?,
            table_hash: reader.read_u32::<LittleEndian>()?,
            layout_hash: reader.read_u32::<LittleEndian>()?,
            min_id: reader.read_u32::<LittleEndian>()?,
            max_id: reader.read_u32::<LittleEndian>()?,
            locale: Locale::try_from(reader.read_u32::<LittleEndian>()?)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?,
            copy_table_size: reader.read_u32::<LittleEndian>()?,
            flags: reader.read_u16::<LittleEndian>()?,
            id_index: reader.read_u16::<LittleEndian>()?,
            total_field_count: reader.read_u32::<LittleEndian>()?,
            bitpacked_data_offset: reader.read_u32::<LittleEndian>()?,
            lookup_column_count: reader.read_u32::<LittleEndian>()?,
            offset_map_offset: reader.read_u32::<LittleEndian>()?,
            id_list_size: reader.read_u32::<LittleEndian>()?,
            field_storage_info_size: reader.read_u32::<LittleEndian>()?,
            common_data_size: reader.read_u32::<LittleEndian>()?,
            pallet_data_size: reader.read_u32::<LittleEndian>()?,
            relationship_data_size: reader.read_u32::<LittleEndian>()?,
        })
    }
}
pub struct DC2Header {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_table_size: u32,
    pub table_hash: u32,
    pub layout_hash: u32,
    pub min_id: u32,
    pub max_id: u32,
    pub locale: Locale,
    pub flags: u16,
    pub id_index: u16,
    pub total_field_count: u32,
    pub bitpacked_data_offset: u32,
    pub lookup_column_count: u32,
    pub field_storage_info_size: u32,
    pub common_data_size: u32,
    pub pallet_data_size: u32,
    pub section_count: u32,
}
impl DC2Header {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            record_count: reader.read_u32::<LittleEndian>()?,
            field_count: reader.read_u32::<LittleEndian>()?,
            record_size: reader.read_u32::<LittleEndian>()?,
            string_table_size: reader.read_u32::<LittleEndian>()?,
            table_hash: reader.read_u32::<LittleEndian>()?,
            layout_hash: reader.read_u32::<LittleEndian>()?,
            min_id: reader.read_u32::<LittleEndian>()?,
            max_id: reader.read_u32::<LittleEndian>()?,
            locale: Locale::try_from(reader.read_u32::<LittleEndian>()?)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?,
            flags: reader.read_u16::<LittleEndian>()?,
            id_index: reader.read_u16::<LittleEndian>()?,
            total_field_count: reader.read_u32::<LittleEndian>()?,
            bitpacked_data_offset: reader.read_u32::<LittleEndian>()?,
            lookup_column_count: reader.read_u32::<LittleEndian>()?,
            field_storage_info_size: reader.read_u32::<LittleEndian>()?,
            common_data_size: reader.read_u32::<LittleEndian>()?,
            pallet_data_size: reader.read_u32::<LittleEndian>()?,
            section_count: reader.read_u32::<LittleEndian>()?,
        })
    }
}
pub struct DC3Header {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_table_size: u32,
    pub table_hash: u32,
    pub layout_hash: u32,
    pub min_id: u32,
    pub max_id: u32,
    pub locale: Locale,
    pub flags: u16,
    pub id_index: u16,
    pub total_field_count: u32,
    pub bitpacked_data_offset: u32,
    pub lookup_column_count: u32,
    pub field_storage_info_size: u32,
    pub common_data_size: u32,
    pub pallet_data_size: u32,
    pub section_count: u32,
}
impl DC3Header {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            record_count: reader.read_u32::<LittleEndian>()?,
            field_count: reader.read_u32::<LittleEndian>()?,
            record_size: reader.read_u32::<LittleEndian>()?,
            string_table_size: reader.read_u32::<LittleEndian>()?,
            table_hash: reader.read_u32::<LittleEndian>()?,
            layout_hash: reader.read_u32::<LittleEndian>()?,
            min_id: reader.read_u32::<LittleEndian>()?,
            max_id: reader.read_u32::<LittleEndian>()?,
            locale: Locale::try_from(reader.read_u32::<LittleEndian>()?)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?,
            flags: reader.read_u16::<LittleEndian>()?,
            id_index: reader.read_u16::<LittleEndian>()?,
            total_field_count: reader.read_u32::<LittleEndian>()?,
            bitpacked_data_offset: reader.read_u32::<LittleEndian>()?,
            lookup_column_count: reader.read_u32::<LittleEndian>()?,
            field_storage_info_size: reader.read_u32::<LittleEndian>()?,
            common_data_size: reader.read_u32::<LittleEndian>()?,
            pallet_data_size: reader.read_u32::<LittleEndian>()?,
            section_count: reader.read_u32::<LittleEndian>()?,
        })
    }
}
pub struct DC4Header {
    pub record_count: u32,
    pub field_count: u32,
    pub record_size: u32,
    pub string_table_size: u32,
    pub table_hash: u32,
    pub layout_hash: u32,
    pub min_id: u32,
    pub max_id: u32,
    pub locale: Locale,
    pub flags: u16,
    pub id_index: u16,
    pub total_field_count: u32,
    pub bitpacked_data_offset: u32,
    pub lookup_column_count: u32,
    pub field_storage_info_size: u32,
    pub common_data_size: u32,
    pub pallet_data_size: u32,
    pub section_count: u32,
}
impl DC4Header {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        Ok(Self {
            record_count: reader.read_u32::<LittleEndian>()?,
            field_count: reader.read_u32::<LittleEndian>()?,
            record_size: reader.read_u32::<LittleEndian>()?,
            string_table_size: reader.read_u32::<LittleEndian>()?,
            table_hash: reader.read_u32::<LittleEndian>()?,
            layout_hash: reader.read_u32::<LittleEndian>()?,
            min_id: reader.read_u32::<LittleEndian>()?,
            max_id: reader.read_u32::<LittleEndian>()?,
            locale: Locale::try_from(reader.read_u32::<LittleEndian>()?)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidData))?,
            flags: reader.read_u16::<LittleEndian>()?,
            id_index: reader.read_u16::<LittleEndian>()?,
            total_field_count: reader.read_u32::<LittleEndian>()?,
            bitpacked_data_offset: reader.read_u32::<LittleEndian>()?,
            lookup_column_count: reader.read_u32::<LittleEndian>()?,
            field_storage_info_size: reader.read_u32::<LittleEndian>()?,
            common_data_size: reader.read_u32::<LittleEndian>()?,
            pallet_data_size: reader.read_u32::<LittleEndian>()?,
            section_count: reader.read_u32::<LittleEndian>()?,
        })
    }
}

#[derive(Debug)]
pub struct DBCFile<Rec: Record> {
    pub header: DBCHeader,
    pub records: Arc<[Rec]>,
    //pub string_table: Arc<[CString]>,
}

impl<Rec: Record> DBCFile<Rec> {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let header = DBCHeader::from_reader(reader)?;
        let mut records = Vec::with_capacity(header.record_count as usize);
        let records_bytes = {
            let mut v = vec![0; (header.record_size * header.record_count) as usize];
            reader.read_exact(&mut v)?;
            v
        };
        //debug_assert_eq!(std::mem::size_of::<Rec>(), header.record_size as usize);

        /*
        let string_table: Vec<CString> = {
            let mut v = vec![0; header.string_table_size as usize];
            reader.read_exact(&mut v)?;
            let mut i = 0;
            let mut strings = Vec::new();
            while i < v.len() {
                let Ok(s) = CStr::from_bytes_until_nul(&v[i..]) else {
                    return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                };
                let s = s.to_owned();
                i += s.as_bytes_with_nul().len();
                strings.push(s);
            }
            strings
        };*/

        let string_table: HashMap<u32, CString> = {
            let mut v = vec![0; header.string_table_size as usize];
            reader.read_exact(&mut v)?;
            let mut i = 0;
            let mut strings = HashMap::new();
            while i < v.len() {
                let Ok(s) = CStr::from_bytes_until_nul(&v[i..]) else {
                    return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                };
                let s = s.to_owned();
                let len = s.as_bytes_with_nul().len();
                strings.insert(i as u32, s);
                i += len;
            }
            strings
        };

        let mut cursor = Cursor::new(&records_bytes);
        for _ in 0..header.record_count {
            records.push(Rec::from_reader(&mut cursor, &|reader: &mut Cursor<
                &Vec<u8>,
            >| {
                //let a = reader.read_u32::<LittleEndian>()? as usize;

                //Ok(string_table[a].clone())

                Ok(string_table
                    .get(&reader.read_u32::<LittleEndian>()?)
                    .unwrap()
                    .clone())
            })?);
        }
        debug_assert!(cursor.split().1.is_empty());

        Ok(Self {
            header,
            records: records.into(),
            //string_table: string_table.into(),
        })
    }
}

impl<Rec: Record> WDBFile<Rec> for DBCFile<Rec> {
    fn get_records(&self) -> Arc<[Rec]> {
        self.records.clone()
    }
}

pub struct DB2File<Rec: Record> {
    pub header: DB2Header,
    pub indices: Option<Arc<[u32]>>,
    pub string_lengths: Option<Arc<[u16]>>,
    pub records: Arc<[Rec]>,
    //pub string_table: Arc<[c_char]>,
}

impl<Rec: Record> DB2File<Rec> {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let header = DB2Header::from_reader(reader)?;

        let (indices, string_lengths) = if header.max_id != 0 {
            let indices: Vec<u32> = {
                let mut v = vec![0; (header.max_id - header.min_id + 1) as usize];
                reader.read_exact(&mut v)?;
                let p = v.into_raw_parts();
                unsafe { Vec::from_raw_parts(p.0 as *mut u32, p.1, p.2) }
            };
            let string_lengths: Vec<u16> = {
                let mut v = vec![0; (header.max_id - header.min_id + 1) as usize];
                reader.read_exact(&mut v)?;
                let p = v.into_raw_parts();
                unsafe { Vec::from_raw_parts(p.0 as *mut u16, p.1, p.2) }
            };
            (Some(indices.into()), Some(string_lengths.into()))
        } else {
            (None, None)
        };

        let mut records = Vec::with_capacity(header.record_count as usize);
        let records_bytes = {
            let mut v = vec![0; (header.record_size * header.record_count) as usize];
            reader.read_exact(&mut v)?;
            v
        };
        debug_assert_eq!(std::mem::size_of::<Rec>(), header.record_size as usize);

        let string_table: Vec<CString> = {
            let mut v = vec![0; header.string_table_size as usize];
            reader.read_exact(&mut v)?;
            let mut i = 0;
            let mut strings = Vec::new();
            while i < v.len() {
                let Ok(s) = CStr::from_bytes_until_nul(&v) else {
                    return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                };
                let s = s.to_owned();
                i += s.as_bytes_with_nul().len();
                strings.push(s);
            }
            strings
        };

        let mut cursor = Cursor::new(&records_bytes);
        for _ in 0..header.record_count {
            records.push(Rec::from_reader(&mut cursor, &|reader: &mut Cursor<
                &Vec<u8>,
            >| {
                Ok(string_table[reader.read_u32::<LittleEndian>()? as usize].clone())
            })?);
        }
        debug_assert!(cursor.split().1.is_empty());

        Ok(Self {
            header,
            indices,
            string_lengths,
            records: records.into(),
            //string_table: string_table.into(),
        })
    }
}

impl<Rec: Record> WDBFile<Rec> for DB2File<Rec> {
    fn get_records(&self) -> Arc<[Rec]> {
        self.records.clone()
    }
}

pub struct DB3File<Rec: Record> {
    pub header: DB2Header,
    pub offset_map: Arc<[OffsetMapEntry]>,
    pub relationship_ids: Arc<[u32]>,
    pub records: Arc<[Rec]>,
    //pub string_table: Arc<[c_char]>,
    pub ids: Arc<[u32]>,
    pub copy_table: Option<Arc<[CopyTableEntry]>>,
}

impl<Rec: Record> DB3File<Rec> {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let header = DB2Header::from_reader(reader)?;

        let mut offset_map = Vec::with_capacity((header.max_id - header.min_id + 1) as usize);
        for _ in 0..((header.max_id - header.min_id + 1) as usize) {
            let offset = reader.read_u32::<LittleEndian>()?;
            let length = reader.read_u16::<LittleEndian>()?;
            offset_map.push(OffsetMapEntry { offset, length });
        }
        let mut relationship_ids = Vec::with_capacity((header.max_id - header.min_id + 1) as usize);
        for _ in 0..((header.max_id - header.min_id + 1) as usize) {
            let id = reader.read_u32::<LittleEndian>()?;
            relationship_ids.push(id);
        }

        let mut records = Vec::with_capacity(header.record_count as usize);
        let records_bytes = {
            let mut v = vec![0; (header.record_size * header.record_count) as usize];
            reader.read_exact(&mut v)?;
            v
        };
        debug_assert_eq!(std::mem::size_of::<Rec>(), header.record_size as usize);

        let string_table: Vec<CString> = {
            let mut v = vec![0; header.string_table_size as usize];
            reader.read_exact(&mut v)?;
            let mut i = 0;
            let mut strings = Vec::new();
            while i < v.len() {
                let Ok(s) = CStr::from_bytes_until_nul(&v) else {
                    return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                };
                let s = s.to_owned();
                i += s.as_bytes_with_nul().len();
                strings.push(s);
            }
            strings
        };

        let mut cursor = Cursor::new(&records_bytes);
        for _ in 0..header.record_count {
            records.push(Rec::from_reader(&mut cursor, &|reader: &mut Cursor<
                &Vec<u8>,
            >| {
                Ok(string_table[reader.read_u32::<LittleEndian>()? as usize].clone())
            })?);
        }
        debug_assert!(cursor.split().1.is_empty());

        let mut ids = Vec::with_capacity(header.record_count as usize);
        for _ in 0..(header.record_count as usize) {
            let id = reader.read_u32::<LittleEndian>()?;
            ids.push(id);
        }
        let copy_table = if header.copy_table_size > 0 {
            let size = (header.copy_table_size as usize) / std::mem::size_of::<CopyTableEntry>();
            let mut copy_table = Vec::with_capacity(size);
            for _ in 0..size {
                let id_of_new_row = reader.read_u32::<LittleEndian>()?;
                let id_of_copied_row = reader.read_u32::<LittleEndian>()?;
                copy_table.push(CopyTableEntry {
                    id_of_new_row,
                    id_of_copied_row,
                });
            }
            Some(copy_table.into())
        } else {
            None
        };

        Ok(Self {
            header,
            offset_map: offset_map.into(),
            relationship_ids: relationship_ids.into(),
            records: records.into(),
            ids: ids.into(),
            copy_table, //string_table: string_table.into(),
        })
    }
}

impl<Rec: Record> WDBFile<Rec> for DB3File<Rec> {
    fn get_records(&self) -> Arc<[Rec]> {
        self.records.clone()
    }
}
pub struct OffsetMapEntry {
    pub offset: u32,
    pub length: u16,
}
pub struct CopyTableEntry {
    pub id_of_new_row: u32,
    pub id_of_copied_row: u32,
}

pub struct DB4File<Rec: Record> {
    pub header: DB4Header,
    pub records: Arc<[Rec]>,
    //pub string_table: Arc<[c_char]>,
    pub offset_map: Option<Arc<[OffsetMapEntry]>>,
    pub relationship_ids: Option<Arc<[u32]>>,
    pub ids: Option<Arc<[u32]>>,
    pub copy_table: Option<Arc<[CopyTableEntry]>>,
}

impl<Rec: Record> DB4File<Rec> {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let header = DB4Header::from_reader(reader)?;

        let mut records = Vec::with_capacity(header.record_count as usize);
        let records_bytes = {
            let mut v = vec![0; (header.record_size * header.record_count) as usize];
            reader.read_exact(&mut v)?;
            v
        };
        debug_assert_eq!(std::mem::size_of::<Rec>(), header.record_size as usize);

        let string_table: Vec<CString> = {
            let mut v = vec![0; header.string_table_size as usize];
            reader.read_exact(&mut v)?;
            let mut i = 0;
            let mut strings = Vec::new();
            while i < v.len() {
                let Ok(s) = CStr::from_bytes_until_nul(&v) else {
                    return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                };
                let s = s.to_owned();
                i += s.as_bytes_with_nul().len();
                strings.push(s);
            }
            strings
        };

        let mut cursor = Cursor::new(&records_bytes);
        for _ in 0..header.record_count {
            records.push(Rec::from_reader(&mut cursor, &|reader: &mut Cursor<
                &Vec<u8>,
            >| {
                Ok(string_table[reader.read_u32::<LittleEndian>()? as usize].clone())
            })?);
        }
        debug_assert!(cursor.split().1.is_empty());

        let offset_map = if header.flags & 0x01 != 0 {
            let mut offset_map = Vec::with_capacity((header.max_id - header.min_id + 1) as usize);
            for _ in 0..((header.max_id - header.min_id + 1) as usize) {
                let offset = reader.read_u32::<LittleEndian>()?;
                let length = reader.read_u16::<LittleEndian>()?;
                offset_map.push(OffsetMapEntry { offset, length });
            }
            Some(offset_map.into())
        } else {
            None
        };
        let relationship_ids = if header.flags & 0x02 != 0 {
            let mut relationship_ids =
                Vec::with_capacity((header.max_id - header.min_id + 1) as usize);
            for _ in 0..((header.max_id - header.min_id + 1) as usize) {
                let id = reader.read_u32::<LittleEndian>()?;
                relationship_ids.push(id);
            }
            Some(relationship_ids.into())
        } else {
            None
        };
        let ids = if header.flags & 0x04 != 0 {
            let mut ids = Vec::with_capacity(header.record_count as usize);
            for _ in 0..(header.record_count as usize) {
                let id = reader.read_u32::<LittleEndian>()?;
                ids.push(id);
            }
            Some(ids.into())
        } else {
            None
        };
        let copy_table = if header.copy_table_size > 0 {
            let size = (header.copy_table_size as usize) / std::mem::size_of::<CopyTableEntry>();
            let mut copy_table = Vec::with_capacity(size);
            for _ in 0..size {
                let id_of_new_row = reader.read_u32::<LittleEndian>()?;
                let id_of_copied_row = reader.read_u32::<LittleEndian>()?;
                copy_table.push(CopyTableEntry {
                    id_of_new_row,
                    id_of_copied_row,
                });
            }
            Some(copy_table.into())
        } else {
            None
        };

        Ok(Self {
            header,
            records: records.into(),
            offset_map,
            relationship_ids,
            ids,
            copy_table, //string_table: string_table.into(),
        })
    }
}

impl<Rec: Record> WDBFile<Rec> for DB4File<Rec> {
    fn get_records(&self) -> Arc<[Rec]> {
        self.records.clone()
    }
}

pub struct DB5File<Rec: Record> {
    pub header: DB5Header,
    pub fields: Arc<[FieldStructure]>,
    pub records: Arc<[Rec]>,
    //pub string_table: Arc<[c_char]>,
    pub offset_map: Option<Arc<[OffsetMapEntry]>>,
    pub relationship_ids: Option<Arc<[u32]>>,
    pub ids: Option<Arc<[u32]>>,
    pub copy_table: Option<Arc<[CopyTableEntry]>>,
}

impl<Rec: Record> DB5File<Rec> {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let header = DB5Header::from_reader(reader)?;

        let mut fields = Vec::new();
        for _ in 0..header.field_count as usize {
            let size = reader.read_u16::<LittleEndian>()?;
            let position = reader.read_u16::<LittleEndian>()?;
            fields.push(FieldStructure { size, position });
        }

        let mut records = Vec::with_capacity(header.record_count as usize);
        let records_bytes = {
            let mut v = vec![0; (header.record_size * header.record_count) as usize];
            reader.read_exact(&mut v)?;
            v
        };
        debug_assert_eq!(std::mem::size_of::<Rec>(), header.record_size as usize);

        let string_table: Vec<CString> = {
            let mut v = vec![0; header.string_table_size as usize];
            reader.read_exact(&mut v)?;
            let mut i = 0;
            let mut strings = Vec::new();
            while i < v.len() {
                let Ok(s) = CStr::from_bytes_until_nul(&v) else {
                    return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                };
                let s = s.to_owned();
                i += s.as_bytes_with_nul().len();
                strings.push(s);
            }
            strings
        };

        let mut cursor = Cursor::new(&records_bytes);
        for _ in 0..header.record_count {
            records.push(Rec::from_reader(&mut cursor, &|reader: &mut Cursor<
                &Vec<u8>,
            >| {
                Ok(string_table[reader.read_u32::<LittleEndian>()? as usize].clone())
            })?);
        }
        debug_assert!(cursor.split().1.is_empty());

        let offset_map = if header.flags & 0x01 != 0 {
            let mut offset_map = Vec::with_capacity((header.max_id - header.min_id + 1) as usize);
            for _ in 0..((header.max_id - header.min_id + 1) as usize) {
                let offset = reader.read_u32::<LittleEndian>()?;
                let length = reader.read_u16::<LittleEndian>()?;
                offset_map.push(OffsetMapEntry { offset, length });
            }
            Some(offset_map.into())
        } else {
            None
        };
        let relationship_ids = if header.flags & 0x02 != 0 {
            let mut relationship_ids =
                Vec::with_capacity((header.max_id - header.min_id + 1) as usize);
            for _ in 0..((header.max_id - header.min_id + 1) as usize) {
                let id = reader.read_u32::<LittleEndian>()?;
                relationship_ids.push(id);
            }
            Some(relationship_ids.into())
        } else {
            None
        };
        let ids = if header.flags & 0x04 != 0 {
            let mut ids = Vec::with_capacity(header.record_count as usize);
            for _ in 0..(header.record_count as usize) {
                let id = reader.read_u32::<LittleEndian>()?;
                ids.push(id);
            }
            Some(ids.into())
        } else {
            None
        };
        let copy_table = if header.copy_table_size > 0 {
            let size = (header.copy_table_size as usize) / std::mem::size_of::<CopyTableEntry>();
            let mut copy_table = Vec::with_capacity(size);
            for _ in 0..size {
                let id_of_new_row = reader.read_u32::<LittleEndian>()?;
                let id_of_copied_row = reader.read_u32::<LittleEndian>()?;
                copy_table.push(CopyTableEntry {
                    id_of_new_row,
                    id_of_copied_row,
                });
            }
            Some(copy_table.into())
        } else {
            None
        };

        Ok(Self {
            header,
            fields: fields.into(),
            records: records.into(),
            offset_map,
            relationship_ids,
            ids,
            copy_table, //string_table: string_table.into(),
        })
    }
}

impl<Rec: Record> WDBFile<Rec> for DB5File<Rec> {
    fn get_records(&self) -> Arc<[Rec]> {
        self.records.clone()
    }
}

pub struct FieldStructure {
    pub size: u16,
    pub position: u16,
}
pub struct DB6File<Rec: Record> {
    pub header: DB6Header,
    pub fields: Arc<[FieldStructure]>,
    pub records: Arc<[Rec]>,
    //pub string_table: Arc<[c_char]>,
    pub offset_map: Option<Arc<[OffsetMapEntry]>>,
    pub relationship_ids: Option<Arc<[u32]>>,
    pub ids: Option<Arc<[u32]>>,
    pub copy_table: Option<Arc<[CopyTableEntry]>>,
    pub common_data_table: Option<Arc<[CommonDataTableEntry]>>,
}

impl<Rec: Record> DB6File<Rec> {
    pub fn from_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let header = DB6Header::from_reader(reader)?;

        let mut fields = Vec::new();
        for _ in 0..header.field_count as usize {
            let size = reader.read_u16::<LittleEndian>()?;
            let position = reader.read_u16::<LittleEndian>()?;
            fields.push(FieldStructure { size, position });
        }

        let mut records = Vec::with_capacity(header.record_count as usize);
        let records_bytes = {
            let mut v = vec![0; (header.record_size * header.record_count) as usize];
            reader.read_exact(&mut v)?;
            v
        };
        debug_assert_eq!(std::mem::size_of::<Rec>(), header.record_size as usize);

        let string_table: Vec<CString> = {
            let mut v = vec![0; header.string_table_size as usize];
            reader.read_exact(&mut v)?;
            let mut i = 0;
            let mut strings = Vec::new();
            while i < v.len() {
                let Ok(s) = CStr::from_bytes_until_nul(&v) else {
                    return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                };
                let s = s.to_owned();
                i += s.as_bytes_with_nul().len();
                strings.push(s);
            }
            strings
        };

        let mut cursor = Cursor::new(&records_bytes);
        for _ in 0..header.record_count {
            records.push(Rec::from_reader(&mut cursor, &|reader: &mut Cursor<
                &Vec<u8>,
            >| {
                Ok(string_table[reader.read_u32::<LittleEndian>()? as usize].clone())
            })?);
        }
        debug_assert!(cursor.split().1.is_empty());

        let offset_map = if header.flags & 0x01 != 0 {
            let mut offset_map = Vec::with_capacity((header.max_id - header.min_id + 1) as usize);
            for _ in 0..((header.max_id - header.min_id + 1) as usize) {
                let offset = reader.read_u32::<LittleEndian>()?;
                let length = reader.read_u16::<LittleEndian>()?;
                offset_map.push(OffsetMapEntry { offset, length });
            }
            Some(offset_map.into())
        } else {
            None
        };
        let relationship_ids = if header.flags & 0x02 != 0 {
            let mut relationship_ids =
                Vec::with_capacity((header.max_id - header.min_id + 1) as usize);
            for _ in 0..((header.max_id - header.min_id + 1) as usize) {
                let id = reader.read_u32::<LittleEndian>()?;
                relationship_ids.push(id);
            }
            Some(relationship_ids.into())
        } else {
            None
        };
        let ids = if header.flags & 0x04 != 0 {
            let mut ids = Vec::with_capacity(header.record_count as usize);
            for _ in 0..(header.record_count as usize) {
                let id = reader.read_u32::<LittleEndian>()?;
                ids.push(id);
            }
            Some(ids.into())
        } else {
            None
        };
        let copy_table = if header.copy_table_size > 0 {
            let size = (header.copy_table_size as usize) / std::mem::size_of::<CopyTableEntry>();
            let mut copy_table = Vec::with_capacity(size);
            for _ in 0..size {
                let id_of_new_row = reader.read_u32::<LittleEndian>()?;
                let id_of_copied_row = reader.read_u32::<LittleEndian>()?;
                copy_table.push(CopyTableEntry {
                    id_of_new_row,
                    id_of_copied_row,
                });
            }
            Some(copy_table.into())
        } else {
            None
        };
        let common_data_table = if header.common_data_table_size > 0 {
            let num_columns_in_table = reader.read_u32::<LittleEndian>()?;
            let mut common_data_table = Vec::with_capacity(num_columns_in_table as usize);
            for _ in 0..num_columns_in_table {
                let count = reader.read_u32::<LittleEndian>()?;
                let ty = reader.read_u8()?;
                let mut common_data_map = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    common_data_map.push(CommonDataMapEntry {
                        id: reader.read_u32::<LittleEndian>()?,
                        value: reader.read_u32::<LittleEndian>()?,
                    });
                }
                common_data_table.push(CommonDataTableEntry {
                    count,
                    ty,
                    common_data_map: common_data_map.into(),
                })
            }
            Some(common_data_table.into())
        } else {
            None
        };

        Ok(Self {
            header,
            fields: fields.into(),
            records: records.into(),
            offset_map,
            relationship_ids,
            ids,
            copy_table,
            common_data_table, //string_table: string_table.into(),
        })
    }
}

impl<Rec: Record> WDBFile<Rec> for DB6File<Rec> {
    fn get_records(&self) -> Arc<[Rec]> {
        self.records.clone()
    }
}

pub struct CommonDataMapEntry {
    pub id: u32,
    pub value: u32,
}
pub struct CommonDataTableEntry {
    pub count: u32,
    pub ty: u8,
    pub common_data_map: Arc<[CommonDataMapEntry]>,
}

pub struct DC1File<Rec: Record> {
    pub header: DC1Header,
    pub fields: Arc<[FieldStructure]>,

    pub records: Arc<[Rec]>,

    pub id_list: Arc<[u32]>,
    pub copy_table: Option<Arc<[CopyTableEntry]>>,
    pub field_info: Arc<[FieldStorageInfoDC1]>,
    pub pallet_data: Arc<[u8]>,
    pub common_data: Arc<[u8]>,
    pub relationship_map: Option<RelationshipMapping>,
}
pub enum FieldCompressionDC1 {
    None {
        unk_or_unused1: u32,
        unk_or_unused2: u32,
        unk_or_unused3: u32,
    },
    Bitpacked {
        bitpacking_offset_bits: u32,
        bitpacking_size_bits: u32,
        flags: u32,
    },
    CommonData {
        default_value: u32,
        unk_or_unused2: u32,
        unk_or_unused3: u32,
    },
    BitpackedIndexed {
        bitpacking_offset_bits: u32,
        bitpacking_size_bits: u32,
        unk_or_unused3: u32,
    },
    BitpackedIndexedArray {
        bitpacking_offset_bits: u32,
        bitpacking_size_bits: u32,
        array_count: u32,
    },
}
pub struct FieldStorageInfoDC1 {
    pub field_offset_bits: u16,
    pub field_size_bits: u16,
    pub additional_data_size: u32,
    pub storage_type: FieldCompressionDC1,
}
pub struct RelationshipMapping {
    pub num_entries: u32,
    pub min_id: u32,
    pub max_id: u32,
    pub entries: Arc<[RelationshipEntry]>,
}
pub struct RelationshipEntry {
    pub foreign_id: u32,
    pub record_index: u32,
}
pub struct DC2File<Rec: Record> {
    pub header: DC2Header,
    pub section_headers: Arc<[DC2SectionHeader]>,
    pub fields: Arc<[FieldStructure]>,
    pub field_info: Arc<[FieldStorageInfoDC2]>,
    pub pallet_data: Arc<[u8]>,
    pub common_data: Arc<[u8]>,
    pub data_sections: Arc<[SectionDC2<Rec>]>,
}
pub struct DC2SectionHeader {
    pub tact_key_hash: u64,
    pub file_offset: u32,
    pub record_count: u32,
    pub string_table_size: u32,
    pub copy_table_size: u32,
    pub offset_map_offset: u32,
    pub id_list_size: u32,
    pub relationship_data_size: u32,
}
pub enum FieldCompressionDC2 {
    None {
        unk_or_unused1: u32,
        unk_or_unused2: u32,
        unk_or_unused3: u32,
    },
    Bitpacked {
        bitpacking_offset_bits: u32,
        bitpacking_size_bits: u32,
        flags: u32,
    },
    CommonData {
        default_value: u32,
        unk_or_unused2: u32,
        unk_or_unused3: u32,
    },
    BitpackedIndexed {
        bitpacking_offset_bits: u32,
        bitpacking_size_bits: u32,
        unk_or_unused3: u32,
    },
    BitpackedIndexedArray {
        bitpacking_offset_bits: u32,
        bitpacking_size_bits: u32,
        array_count: u32,
    },
    BitpackedSigned {
        bitpacking_offset_bits: u32,
        bitpacking_size_bits: u32,
        flags: u32,
    },
}
pub struct FieldStorageInfoDC2 {
    pub field_offset_bits: u16,
    pub field_size_bits: u16,
    pub additional_data_size: u32,
    pub storage_type: FieldCompressionDC2,
}
pub struct SectionDC2<Rec: Record> {
    pub records: Arc<[Rec]>,
    pub id_list: Arc<[u32]>,
    pub copy_table: Option<Arc<[CopyTableEntry]>>,
    pub offset_map: Arc<[OffsetMapEntry]>,
    pub relationship_map: Option<RelationshipMapping>,
}
pub struct DC3File<Rec: Record> {
    pub header: DC3Header,
    pub section_headers: Arc<[DC3SectionHeader]>,
    pub fields: Arc<[FieldStructure]>,
    pub field_info: Arc<[FieldStorageInfoDC2]>,
    pub pallet_data: Arc<[u8]>,
    pub common_data: Arc<[u8]>,
    pub data_sections: Arc<[SectionDC3<Rec>]>,
}
pub struct DC3SectionHeader {
    pub tact_key_hash: u64,
    pub file_offset: u32,
    pub record_count: u32,
    pub string_table_size: u32,
    pub offset_records_end: u32,
    pub id_list_size: u32,
    pub relationship_data_size: u32,
    pub offset_map_id_count: u32,
    pub copy_table_count: u32,
}
pub struct SectionDC3<Rec: Record> {
    pub records: Arc<[Rec]>,
    pub id_list: Arc<[u32]>,
    pub copy_table: Option<Arc<[CopyTableEntry]>>,
    pub offset_map: Arc<[OffsetMapEntry]>,
    pub relationship_map: Option<RelationshipMapping>,
    pub offset_map_id_list: Arc<[u32]>,
}
pub struct DC4File<Rec: Record> {
    pub header: DC4Header,
    pub section_headers: Arc<[DC3SectionHeader]>,
    pub fields: Arc<[FieldStructure]>,
    pub field_info: Arc<[FieldStorageInfoDC2]>,
    pub pallet_data: Arc<[u8]>,
    pub common_data: Arc<[u8]>,
    pub encrypted_records: Arc<[EncryptedStatus]>,
    pub data_sections: Arc<[SectionDC3<Rec>]>,
}
pub struct EncryptedStatus {
    pub encrypted_id_count: u32,
    pub encrypted_id: Arc<[u32]>,
}

#[derive(Clone, Copy, Debug)]
pub struct StringIndex(pub u32);
#[derive(Debug)]
pub struct LocString<const S: usize> {
    pub locales: [CString; S],
    //pub locales: [StringIndex; S],
    pub flags: u32,
}

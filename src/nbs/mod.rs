use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use crate::{NbsIOError, read_nbs, write_nbs};

mod custom_instrument;
mod header;
mod noteblock;

pub use custom_instrument::*;
pub use header::*;
pub use noteblock::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NbsVersion {
    V5 = 5,
    V4 = 4,
    V3 = 3,
    V2 = 2,
    V1 = 1,
    Classic = 0,
}

impl NbsVersion {
    #[allow(non_upper_case_globals)]
    pub const Latest: NbsVersion = NbsVersion::V5;
}

impl TryFrom<u8> for NbsVersion {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            5 => Ok(NbsVersion::V5),
            4 => Ok(NbsVersion::V4),
            3 => Ok(NbsVersion::V3),
            2 => Ok(NbsVersion::V2),
            1 => Ok(NbsVersion::V1),
            0 => Ok(NbsVersion::Classic),
            _ => Err(value),
        }
    }
}

#[derive(Debug)]
pub struct Nbs {
    pub version: NbsVersion,
    pub header: Header,
    pub note_blocks: NoteBlocks,
    pub instrument_set: InstrumentSet,
}

impl Nbs {
    pub fn new() -> Self {
        Nbs {
            version: NbsVersion::Latest,
            header: Header::default(),
            note_blocks: NoteBlocks::default(),
            instrument_set: InstrumentSet::default(),
        }
    }

    pub fn with_version(version: NbsVersion) -> Self {
        Nbs {
            version,
            header: Header::default(),
            note_blocks: NoteBlocks::default(),
            instrument_set: InstrumentSet::default(),
        }
    }

    pub fn read(reader: &mut impl Read) -> Result<Self, NbsIOError> {
        read_nbs(reader)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, NbsIOError> {
        let mut file = File::open(path)?;
        read_nbs(&mut file)
    }

    pub fn write(&self, writer: &mut impl Write) -> Result<(), NbsIOError> {
        write_nbs(writer, self)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), NbsIOError> {
        let mut file = File::create(path)?;
        write_nbs(&mut file, self)
    }
}

impl Default for Nbs {
    fn default() -> Self {
        Nbs::new()
    }
}

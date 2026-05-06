use crate::nbs::{custom_instrument::CustomInstruments, header::Header, noteblock::NoteBlocks};

pub mod custom_instrument;
pub mod header;
pub mod noteblock;

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
    pub custom_instruments: CustomInstruments,
}

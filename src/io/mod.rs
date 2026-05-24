#[cfg(feature = "midi")]
pub mod midi;
mod reader;
mod writer;
#[cfg(feature = "zip")]
mod zip;

pub use reader::{read_header, read_nbs};
pub use writer::write_nbs;

#[derive(Debug, thiserror::Error)]
pub enum NbsIOError {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u8),
}

macro_rules! nbsver_required {
    ($version: ident = $required: ident, $newer: expr, $older: expr) => {
        if $version == $crate::nbs::NbsVersion::$required {
            $newer
        } else {
            $older
        }
    };
    ($version: ident > $required: ident, $newer: expr, $older: expr) => {
        if $version > $crate::nbs::NbsVersion::$required {
            $newer
        } else {
            $older
        }
    };
    ($version: ident >= $required: ident, $newer: expr, $older: expr) => {
        if $version >= $crate::nbs::NbsVersion::$required {
            $newer
        } else {
            $older
        }
    };
    ($version: ident = $required: ident, $newer: expr) => {
        if $version == $crate::nbs::NbsVersion::$required {
            $newer
        }
    };
    ($version: ident > $required: ident, $newer: expr) => {
        if $version > $crate::nbs::NbsVersion::$required {
            $newer
        }
    };
    ($version: ident >= $required: ident, $newer: expr) => {
        if $version >= $crate::nbs::NbsVersion::$required {
            $newer
        }
    };
}

pub(crate) use nbsver_required;

mod reader;
mod writer;

pub use reader::NbsReader;
pub use writer::NbsWriter;

#[derive(Debug, thiserror::Error)]
pub enum NbsIOError {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u8),
}

#[macro_export(local_inner_macros)]
macro_rules! nbsver_required {
    ($version: ident = $required: ident, $newer: expr, $older: expr) => {
        if $version == crate::nbs::NbsVersion::$required {
            $newer
        } else {
            $older
        }
    };
    ($version: ident > $required: ident, $newer: expr, $older: expr) => {
        if $version > crate::nbs::NbsVersion::$required {
            $newer
        } else {
            $older
        }
    };
    ($version: ident >= $required: ident, $newer: expr, $older: expr) => {
        if $version >= crate::nbs::NbsVersion::$required {
            $newer
        } else {
            $older
        }
    };
}

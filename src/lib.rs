#[cfg(feature = "audio")]
pub mod audio;
pub mod header;
pub mod instrument;
pub mod io;
mod nbs;
pub mod noteblock;

pub use nbs::*;

pub type Tick = u32;

#[cfg(test)]
mod tests {}

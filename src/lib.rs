#[cfg(feature = "audio")]
pub mod audio;
pub mod header;
mod instrument;
mod io;
mod nbs;
pub mod noteblock;

pub use instrument::Instrument;
pub use io::*;
pub use nbs::*;

pub type Tick = u32;

#[cfg(test)]
mod tests {}

#[cfg(feature = "audio")]
pub mod audio;
mod instrument;
mod io;
pub mod nbs;

pub use instrument::Instrument;
pub use io::*;
pub use nbs::Nbs;

pub type Tick = u32;

#[cfg(test)]
mod tests {}

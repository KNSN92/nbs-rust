pub mod instrument;
mod mixer;
pub mod note;
mod renderer;
mod resampler;
mod stream;
mod tempo;

use std::num::{NonZeroU16, NonZeroU32};

pub use renderer::*;
pub use stream::*;
pub use tempo::TempoMap;

pub type Frame = [f32; 2]; // Nbs sound is stereo, so 2 channels
pub type SampleRate = NonZeroU32;
pub type Channels = NonZeroU16;

pub const HZ_44100: SampleRate = unsafe { NonZeroU32::new_unchecked(44100) };
pub const HZ_48000: SampleRate = unsafe { NonZeroU32::new_unchecked(48000) };

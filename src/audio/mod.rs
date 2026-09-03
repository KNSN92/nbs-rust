mod decoder;
mod instrument_audio;
mod mixer;
mod note_audio;
pub mod provider;
mod renderer;
mod resampler;
mod stream;
mod tempo;
mod vanilla_audio;

use std::num::{NonZeroU16, NonZeroU32};

pub use instrument_audio::InstrumentAudio;
pub use note_audio::*;
pub use renderer::NbsAudioRenderer;
pub use stream::*;
pub use tempo::TempoMap;

pub type Frame = [f32; 2]; // Nbs sound is stereo, so 2 channels
pub type SampleRate = NonZeroU32;
pub type Channels = NonZeroU16;

pub const HZ_44100: SampleRate = unsafe { NonZeroU32::new_unchecked(44100) };
pub const HZ_48000: SampleRate = unsafe { NonZeroU32::new_unchecked(48000) };

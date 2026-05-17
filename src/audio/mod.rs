mod decoder;
mod instrument_audio;
mod note_audio;
pub mod provider;
mod renderer;
mod vanilla_audio;

use std::num::{NonZeroU16, NonZeroU32};

pub use instrument_audio::InstrumentAudio;
pub use note_audio::NoteAudio;
pub use renderer::NbsAudioRenderer;

pub type Float = f32;
pub type Sample = Float;
pub type Frame = [Sample; 2]; // Nbs sound is stereo, so 2 channels
pub type SampleRate = NonZeroU32;
pub type Channels = NonZeroU16;

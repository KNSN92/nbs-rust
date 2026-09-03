mod decoder;
mod provider;
mod vanilla;

pub use decoder::*;
pub use provider::*;
pub use vanilla::VANILLA_AUDIOS;

use std::{fs::File, io::Cursor, num::NonZeroU32, time::Duration};

use crate::audio::{Channels, Frame, Frames, SampleRate};

#[derive(Debug, Clone)]
pub struct InstrumentAudio(Frames);

impl InstrumentAudio {
    pub fn from_file(
        file: File,
        hint_ext: Option<&str>,
    ) -> Result<InstrumentAudio, DecodeAudioError> {
        decode_audio(file, hint_ext)
    }

    pub fn from_bytes(
        data: impl AsRef<[u8]> + Send + Sync + 'static,
        hint_ext: Option<&str>,
    ) -> Result<InstrumentAudio, DecodeAudioError> {
        decode_audio(Cursor::new(data), hint_ext)
    }

    pub fn new(samples: &[f32], channels: Channels, sample_rate: SampleRate) -> Self {
        InstrumentAudio(Frames::from_samples(samples, channels, sample_rate))
    }

    #[inline]
    pub fn sample_rate(&self) -> NonZeroU32 {
        self.0.sample_rate()
    }

    #[inline]
    pub fn frames(&self) -> &[Frame] {
        &self.0
    }

    #[inline]
    pub fn frame_count(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.0.len() as f64 / self.0.sample_rate().get() as f64)
    }
}

use std::{fs::File, io::Cursor, num::NonZeroU32, sync::Arc, time::Duration};

use crate::audio::{
    Channels, Frame, SampleRate,
    decoder::{DecodeAudioError, decode_audio},
};

#[derive(Debug)]
pub struct InstrumentAudio {
    frames: Arc<[Frame]>, // I'm aiming for multi-threaded audio rendering, so using Arc
    sample_rate: SampleRate,
    pos: usize,
}

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

    pub fn new(
        samples: impl Into<Vec<f32>>,
        channels: Channels,
        sample_rate: SampleRate,
    ) -> Self {
        let frames = to_stereo(samples.into(), channels).into();
        InstrumentAudio {
            frames,
            sample_rate,
            pos: 0,
        }
    }

    #[inline]
    pub fn sample_rate(&self) -> NonZeroU32 {
        self.sample_rate
    }

    #[inline]
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    #[inline]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    #[inline]
    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.frames.len() as f64 / self.sample_rate.get() as f64)
    }
}

fn to_stereo(audio: Vec<f32>, channels: Channels) -> Vec<Frame> {
    let channels = channels.get() as usize;
    let mut frames = Vec::with_capacity(audio.len() / channels * 2);
    for frame in audio.chunks(channels) {
        let frame = match frame {
            [s] => [*s, *s],
            [l, r] => [*l, *r],
            [l, r, ..] => [*l, *r],
            [] => break,
        };
        frames.push(frame)
    }
    frames
}

impl Clone for InstrumentAudio {
    fn clone(&self) -> Self {
        InstrumentAudio {
            frames: self.frames.clone(),
            sample_rate: self.sample_rate,
            pos: 0,
        }
    }
}

impl Iterator for InstrumentAudio {
    type Item = Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let frame = self.frames.get(self.pos).copied();
        if frame.is_some() {
            self.pos += 1;
        }
        frame
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.frames.len() - self.pos;
        (remaining, Some(remaining))
    }
}

mod decoder;
pub mod instrument;
mod mixer;
pub mod note_audio;
mod renderer;
pub mod resampler;
pub mod stream;
mod tempo;

use std::{
    fs::File,
    io::Cursor,
    iter::repeat_n,
    num::{NonZeroU16, NonZeroU32},
    sync::Arc,
};

pub use renderer::*;
pub use stream::*;
pub use tempo::TempoMap;
use wide::f32x16;

use crate::audio::decoder::{DecodeAudioError, decode_audio};

pub type SampleRate = NonZeroU32;
pub type Channels = NonZeroU16;

pub const HZ_44100: SampleRate = unsafe { NonZeroU32::new_unchecked(44100) };
pub const HZ_48000: SampleRate = unsafe { NonZeroU32::new_unchecked(48000) };

pub type Frame = [f32; 2]; // Nbs sound is stereo, so 2 channels

#[derive(Debug, Clone)]
pub struct AudioBuffer(Arc<[Frame]>, usize, SampleRate);

impl AudioBuffer {
    pub fn from_samples(samples: &[f32], channels: Channels, sample_rate: SampleRate) -> Self {
        let channels = channels.get() as usize;
        let mut frames = Vec::with_capacity(samples.len().div_ceil(channels) * 2);
        for frame in samples.chunks(channels) {
            let frame = match frame {
                [s] => [*s, *s],
                [l, r] => [*l, *r],
                [l, r, ..] => [*l, *r],
                [] => break,
            };
            frames.push(frame)
        }
        AudioBuffer::from_vec(frames, sample_rate)
    }

    pub fn from_vec(frames: impl Into<Vec<Frame>>, sample_rate: SampleRate) -> Self {
        let mut frames = frames.into();
        let mut len = frames.len();
        //* 8フレーム分のパディングを追加する。ただし、最後の8フレームがすでに0.0で埋まっている場合はそれを利用し、lenを8減らす。これにより、SIMDでの読み取り時にバッファオーバーフローチェックが不要になる。
        const EMPTY_CHUNK: [Frame; 8] = [[0.0, 0.0]; 8];
        match frames.last_chunk::<8>() {
            Some(&EMPTY_CHUNK) => len -= 8,
            _ => frames.extend(repeat_n([0.0, 0.0], 8)),
        }
        AudioBuffer(frames.into(), len, sample_rate)
    }

    pub fn from_file(file: File, hint_ext: Option<&str>) -> Result<Self, DecodeAudioError> {
        decode_audio(file, hint_ext)
    }

    pub fn from_file_bytes(
        data: impl AsRef<[u8]> + Send + Sync + 'static,
        hint_ext: Option<&str>,
    ) -> Result<Self, DecodeAudioError> {
        decode_audio(Cursor::new(data), hint_ext)
    }

    pub fn sample_rate(&self) -> SampleRate {
        self.2
    }

    pub fn len(&self) -> usize {
        self.1
    }

    pub fn actual_len(&self) -> usize {
        self.0.len()
    }

    pub fn as_slice(&self) -> &[Frame] {
        &self.0[..self.1]
    }

    #[inline(always)]
    pub fn get_frame(&self, index: usize) -> Option<&Frame> {
        if index < self.1 {
            Some(&self.0[index])
        } else {
            None
        }
    }

    #[inline(always)]
    pub(crate) fn get_chunk_simd(&self, index: usize) -> Option<f32x16> {
        if index < self.1 {
            let chunk = unsafe {
                //* 8フレーム分のパディングがあるため、indexがlen未満であれば、index番目以降の8フレームは有効な範囲内にある。
                let frames_ptr = self.0.as_ptr().add(index).cast::<f32x16>();
                //* f32x16は64-byteアライメントが行われているため、read_unalignedを使用する必要がある。
                frames_ptr.read_unaligned()
            };
            Some(chunk)
        } else {
            None
        }
    }
}

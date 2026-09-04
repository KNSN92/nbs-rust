use std::{iter::repeat_n, ops::Deref, sync::Arc};

use crate::audio::{Channels, SampleRate};

pub type Frame = [f32; 2]; // Nbs sound is stereo, so 2 channels

#[derive(Debug, Clone)]
pub struct Frames(Arc<[Frame]>, usize, SampleRate);

impl Frames {
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
        Frames::from_vec(frames, sample_rate)
    }

    pub fn from_vec(mut frames: Vec<Frame>, sample_rate: SampleRate) -> Self {
        let mut len = frames.len();
        //* 8フレーム分のパディングを追加する。ただし、最後の8フレームがすでに0.0で埋まっている場合はそれを利用し、lenを8減らす。これにより、SIMDでの読み取り時にバッファオーバーフローチェックが不要になる。
        const EMPTY_CHUNK: [Frame; 8] = [[0.0, 0.0]; 8];
        match frames.last_chunk::<8>() {
            Some(&EMPTY_CHUNK) => len -= 8,
            _ => frames.extend(repeat_n([0.0, 0.0], 8)),
        }
        Frames(frames.into(), len, sample_rate)
    }

    pub fn sample_rate(&self) -> SampleRate {
        self.2
    }

    pub(crate) fn as_raw_parts_for_mixer(&self) -> (*const Frame, usize) {
        (self.0.as_ptr(), self.1)
    }
}

impl Deref for Frames {
    type Target = [Frame];

    fn deref(&self) -> &Self::Target {
        &self.0[..self.1]
    }
}

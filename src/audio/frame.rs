use std::{iter::repeat_n, ops::Deref, sync::Arc};

use crate::audio::Channels;

pub type Frame = [f32; 2]; // Nbs sound is stereo, so 2 channels

#[derive(Debug, Clone)]
pub struct Frames(usize, Arc<[Frame]>);

impl Frames {
    pub fn from_vec(mut frames: Vec<Frame>) -> Self {
        let mut len = frames.len();
        //* 8フレーム分のパディングを追加する。ただし、最後の8フレームがすでに0.0で埋まっている場合はそれを利用し、lenを8減らす。これにより、SIMDでの読み取り時にバッファオーバーフローチェックが不要になる。
        const EMPTY_CHUNK: [Frame; 8] = [[0.0, 0.0]; 8];
        match frames.last_chunk::<8>() {
            Some(&EMPTY_CHUNK) => len -= 8,
            _ => frames.extend(repeat_n([0.0, 0.0], 8)),
        }
        Frames(len, frames.into())
    }

    pub fn as_raw_parts(&self) -> (*const Frame, usize) {
        (self.1.as_ptr(), self.0)
    }
}

impl Deref for Frames {
    type Target = [Frame];

    fn deref(&self) -> &Self::Target {
        &self.1[..self.0]
    }
}

pub fn to_stereo(audio: Vec<f32>, channels: Channels) -> Vec<Frame> {
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

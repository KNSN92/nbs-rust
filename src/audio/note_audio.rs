use std::{iter::repeat_n, mem, num::NonZeroU32, ops::Deref, sync::Arc, time::Duration};

use wide::f32x16;

use crate::{
    audio::{Frame, SampleRate},
    instrument::Instrument,
    noteblock::Note,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteAudioKey {
    instrument: Instrument,
    key: u8,
    pitch: i16,
}

impl NoteAudioKey {
    pub fn new(instrument: Instrument, key: u8, pitch: i16) -> Self {
        NoteAudioKey {
            instrument,
            key,
            pitch,
        }
    }
}

impl From<Note> for NoteAudioKey {
    fn from(note: Note) -> Self {
        NoteAudioKey {
            instrument: note.instrument,
            key: note.key,
            pitch: note.pitch,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frames(usize, Arc<[Frame]>);

impl Frames {
    pub fn from_vec(mut frames: Vec<Frame>) -> Self {
        let len = frames.len();
        //* 8フレーム分のパディングを追加する。これにより、SIMDでの読み取り時に境界チェックを行わずに済む。
        frames.extend(repeat_n([0.0, 0.0], 8));
        Frames(len, frames.into())
    }
}

impl Deref for Frames {
    type Target = [Frame];

    fn deref(&self) -> &Self::Target {
        &self.1[..self.0]
    }
}

#[derive(Debug, Clone)]
pub struct NoteAudio {
    frames: Frames,
    multiplier: f32x16,
    sample_rate: SampleRate,
    pos: usize,
}

impl NoteAudio {
    pub fn new(frames: Frames, note: Note, weight: NoteWeight, sample_rate: SampleRate) -> Self {
        NoteAudio {
            frames,
            multiplier: multiplier(&note, weight),
            sample_rate,
            pos: 0,
        }
    }

    #[inline]
    pub fn sample_rate(&self) -> NonZeroU32 {
        self.sample_rate
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.frames.len() as f64 / self.sample_rate.get() as f64)
    }

    pub fn for_note(&self, note: &Note, weight: NoteWeight) -> Self {
        NoteAudio {
            frames: self.frames.clone(),
            multiplier: multiplier(note, weight),
            sample_rate: self.sample_rate,
            pos: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn next_chunk_simd(&mut self) -> Option<f32x16> {
        //* self.frames.0は最後のパディングの長さを含まないため、パディングをframesの一部として境界チェックを行ってしまい、下でバッファオーバーフローが発生する事はない。
        if self.pos >= self.frames.0 {
            return None;
        }
        unsafe {
            // pos番目以降のframesを指すポインタを取得する。
            let frames_ptr = self.frames.1.as_ptr().add(self.pos).cast::<f32x16>();
            self.pos += 8;
            //* framesの最後には8フレーム分(f32 * 16個分)のパディングがあるため、上の境界チェックが正しい限り16個の連続したf32サンプルが有効な範囲内にあります。
            //* f32x16は64-byteアライメントが行われているため、read_unalignedを使用する必要がある。
            Some(frames_ptr.read_unaligned() * self.multiplier)
        }
    }

    pub fn next_chunk(&mut self) -> Option<[Frame; 8]> {
        let chunk = self.next_chunk_simd()?;
        let chunk = unsafe { mem::transmute(chunk.to_array()) };
        Some(chunk)
    }

    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoteWeight {
    pub volume: u8,
    pub panning: u8,
    pub key: u8,
}

impl Default for NoteWeight {
    fn default() -> Self {
        NoteWeight {
            volume: 100,
            panning: 100,
            key: 45,
        }
    }
}

fn multiplier(note: &Note, weight: NoteWeight) -> f32x16 {
    let volume = note.volume(weight);
    let panning = note.panning(weight);
    // Safely transmute the array of 2-element arrays into a 16-element array, since we know the size is correct.
    let multiplier: [f32; 16] =
        unsafe { mem::transmute([[panning[0] * volume, panning[1] * volume]; 8]) };
    f32x16::new(multiplier)
}

impl Iterator for NoteAudio {
    type Item = Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let frame = self.frames.get(self.pos).copied()?;
        let multiplier = self.multiplier.to_array();
        self.pos += 1;
        Some([frame[0] * multiplier[0], frame[1] * multiplier[1]])
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.frames.len() - self.pos;
        (remaining, Some(remaining))
    }
}

impl Note {
    fn volume(&self, weight: NoteWeight) -> f32 {
        let layer_volume = weight.volume as f32 / 100.0;
        let note_volume = self.volume as f32 / 100.0;
        note_volume * layer_volume
    }

    fn panning(&self, weight: NoteWeight) -> [f32; 2] {
        let layer_panning = weight.panning as f32 / 100.0;
        let note_panning = self.panning as f32 / 100.0;
        let panning = match layer_panning {
            0.0 => note_panning,
            _ => (layer_panning + note_panning) / 2.0,
        };
        [2.0 - panning, panning]
    }

    pub(crate) fn pitch(&self, weight: NoteWeight) -> f64 {
        let instrument_key = weight.key as f64 - 45.0;
        let pitch = self.pitch as f64;
        let key = self.key as f64;
        let key = key + instrument_key + pitch / 100.0;
        let key = key - 45.0;
        2.0f64.powf(key / 12.0)
    }
}

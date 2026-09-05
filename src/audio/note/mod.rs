mod provider;

pub use provider::*;

use std::{mem, num::NonZeroU32, time::Duration};

use wide::f32x16;

use crate::{audio::AudioBuffer, instrument::Instrument, noteblock::Note};

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
pub struct NoteAudio {
    frames: AudioBuffer,
    multiplier: f32x16,
}

impl NoteAudio {
    pub fn new(frames: AudioBuffer, note: Note, weight: NoteWeight) -> Self {
        NoteAudio {
            frames,
            multiplier: multiplier(&note, weight),
        }
    }

    #[inline]
    pub fn sample_rate(&self) -> NonZeroU32 {
        self.frames.sample_rate()
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.frames.len() as f64 / self.sample_rate().get() as f64)
    }

    pub fn for_note(&self, note: &Note, weight: NoteWeight) -> Self {
        NoteAudio {
            frames: self.frames.clone(),
            multiplier: multiplier(note, weight),
        }
    }

    pub(crate) fn into_parts(self) -> (AudioBuffer, f32x16) {
        (self.frames, self.multiplier)
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

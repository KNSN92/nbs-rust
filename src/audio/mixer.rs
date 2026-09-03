use std::mem;

use wide::f32x16;

use crate::audio::{Frame, note::NoteAudio};

#[derive(Debug)]
pub struct NoteAudioMixer {
    note_audios: Vec<NoteAudio>,
    chunk: [Frame; 8],
    pos: usize,
}

impl NoteAudioMixer {
    pub fn new() -> Self {
        Self {
            note_audios: Vec::new(),
            chunk: [[0.0, 0.0]; 8],
            pos: 0,
        }
    }

    pub fn mix_note(&mut self, audio: NoteAudio) {
        self.note_audios.push(audio);
    }

    pub fn mixed_notes(&self) -> usize {
        self.note_audios.len()
    }

    pub fn is_empty(&self) -> bool {
        self.note_audios.is_empty()
    }

    pub fn next_chunk(&mut self) -> [Frame; 8] {
        let mut chunk_acc = f32x16::ZERO;
        let mut i = 0;
        while i < self.note_audios.len() {
            if let Some(chunk) = self.note_audios[i].next_chunk_simd() {
                chunk_acc += chunk;
                i += 1;
            } else {
                self.note_audios.swap_remove(i);
            }
        }
        let chunk = unsafe { mem::transmute(chunk_acc) }; // Transmute the f32x16([f32; 16]) back to [[f32; 2]; 8]
        chunk
    }

    pub fn next_frame(&mut self) -> Frame {
        let frame = self.chunk[self.pos];
        self.pos += 1;
        if self.pos >= 8 {
            self.chunk = self.next_chunk();
            self.pos = 0;
        }
        frame
    }

    pub fn fill_buffer(&mut self, buf: &mut [Frame]) {
        let mut i = 0;
        while i < buf.len() {
            let remaining = buf.len() - i;
            if self.pos > 0 || remaining < 8 {
                buf[i] = self.next_frame();
                i += 1;
            } else {
                let chunk = self.next_chunk();
                buf[i..i + 8].copy_from_slice(&chunk);
                i += 8;
            }
        }
    }
}

use std::mem;

use wide::f32x16;

use crate::audio::{AudioBuffer, Frame, note::NoteAudio};

#[derive(Debug)]
struct PlayingNoteAudio {
    audio: AudioBuffer,
    multiplier: f32x16,
    pos: usize,
}

//* framesはArcで管理されており、内部も[Frame]というただのf32の配列であるため、安全にSendを実装出来る。PlyaingNoteAudioはポインタを書き換えることはなく、参照するだけなので、データ競合は発生しない。
unsafe impl Send for PlayingNoteAudio {}

impl PlayingNoteAudio {
    pub fn new(audio: NoteAudio) -> Self {
        let (frames, multiplier) = audio.into_parts();
        PlayingNoteAudio {
            audio: frames,
            multiplier,
            pos: 0,
        }
    }

    #[inline(always)]
    pub(crate) fn next_chunk_simd(&mut self) -> Option<f32x16> {
        let (frames, len) = self.audio.as_raw_parts_for_mixer();
        //* lenは最後のパディングの長さを含まないため、パディングをframesの一部として境界チェックを行ってしまい、下でバッファオーバーフローが発生する事はない。
        if self.pos >= len {
            return None;
        }
        unsafe {
            // pos番目以降のframesを指すポインタを取得する。
            let frames_ptr = frames.add(self.pos).cast::<f32x16>();
            self.pos += 8;
            //* framesの最後には8フレーム分(f32 * 16個分)のパディングがあるため、上の境界チェックが正しい限り16個の連続したf32サンプルが有効な範囲内にあります。
            //* f32x16は64-byteアライメントが行われているため、read_unalignedを使用する必要がある。
            Some(frames_ptr.read_unaligned() * self.multiplier)
        }
    }
}

#[derive(Debug)]
pub struct NoteAudioMixer {
    note_audios: Vec<PlayingNoteAudio>,
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
        self.note_audios.push(PlayingNoteAudio::new(audio));
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

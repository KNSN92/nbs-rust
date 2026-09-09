use crate::audio::{
    Frame, NbsEvent, NbsStream, SampleRate, mixer::NoteAudioMixer, note_audio::NoteAudio,
};

pub struct NbsAudioRenderer<T>
where
    T: NbsStream<NoteAudio>,
{
    note_stream: Option<T>,
    mixer: NoteAudioMixer,
    sample_rate: SampleRate,
    samples_until_next_tick: usize,
    tempo: f32,
}

impl<T> NbsAudioRenderer<T>
where
    T: NbsStream<NoteAudio>,
{
    pub fn new(note_stream: T, sample_rate: SampleRate) -> Self {
        let tempo = note_stream.default_tempo();
        let note_stream = Some(note_stream);
        NbsAudioRenderer {
            note_stream,
            sample_rate,
            samples_until_next_tick: 0,
            tempo,
            mixer: NoteAudioMixer::new(),
        }
    }

    #[inline]
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    pub fn current_tempo(&self) -> f32 {
        self.tempo
    }

    pub fn playing_sounds_count(&self) -> usize {
        self.mixer.mixed_notes()
    }

    //TODO: 曲の長さをDurationで取得する関数を追加したい。

    fn samples_per_tick(&self) -> usize {
        (self.sample_rate().get() as f32 / self.tempo).round() as usize
    }

    fn tick(&mut self) {
        while let Some(note_stream) = &mut self.note_stream {
            match note_stream.next_event() {
                NbsEvent::NotePlay(audio) => self.mixer.mix_note(audio),
                NbsEvent::TempoChange(tempo) => self.tempo = tempo,
                NbsEvent::NoOp => continue,
                NbsEvent::TickAdvance => break,
                NbsEvent::EndOfStream => {
                    self.note_stream = None;
                    break;
                }
            }
        }
    }

    pub fn next_frame(&mut self) -> Option<Frame> {
        if self.samples_until_next_tick == 0 {
            if self.note_stream.is_none() && self.mixer.is_empty() {
                return None;
            }
            self.tick();
            self.samples_until_next_tick = self.samples_per_tick();
        } else {
            self.samples_until_next_tick -= 1;
        }
        Some(self.mixer.next_frame())
    }

    //TODO: 何故か音割れ？になっているのでfixしないといけないよ。どこが悪いのか検討つきませ〜ん:(
    #[allow(unused)]
    fn fill_buffer(&mut self, buf: &mut [Frame]) {
        let mut i = 0;
        while i < buf.len() {
            let remaining = buf.len() - i;
            if remaining >= self.samples_until_next_tick {
                self.mixer
                    .fill_buffer(&mut buf[i..i + self.samples_until_next_tick]);
                i += self.samples_until_next_tick;
                self.tick();
                self.samples_until_next_tick = self.samples_per_tick();
            } else {
                self.mixer.fill_buffer(&mut buf[i..]);
                self.samples_until_next_tick -= remaining;
                break;
            }
        }
    }
}

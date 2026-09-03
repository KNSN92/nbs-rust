use std::num::NonZeroUsize;

use crate::audio::{
    Frame, NoteStream, NoteStreamEvent, SampleRate,
    instrument_provider::InstrumentAudioProvider,
    mixer::NoteAudioMixer,
    noteaudio_provider::{CacheCapacity, NoteAudioMissPolicy, NoteAudioProvider, NumThreads},
    resampler::InterpolationType,
};

pub struct NbsAudioRendererParams {
    pub num_threads: NumThreads,
    pub miss_policy: NoteAudioMissPolicy,
    pub prefetchable_cap: NonZeroUsize,
    pub cache_capacity: CacheCapacity,
    pub interpolation_type: InterpolationType,
}

impl Default for NbsAudioRendererParams {
    fn default() -> Self {
        NbsAudioRendererParams {
            num_threads: NumThreads::default(),
            miss_policy: NoteAudioMissPolicy::SyncFallback,
            prefetchable_cap: 256.try_into().unwrap(),
            cache_capacity: CacheCapacity::Bounded(256.try_into().unwrap()),
            interpolation_type: InterpolationType::Cubic,
        }
    }
}

pub struct NbsAudioRenderer<T>
where
    T: NoteStream,
{
    note_stream: Option<T>,
    audio_provider: NoteAudioProvider,
    mixer: NoteAudioMixer,
    sample_rate: SampleRate,
    miss_policy: NoteAudioMissPolicy,
    prefetchable_cap: NonZeroUsize,
    prefetch_note_stream: Option<T>,
    samples_until_next_tick: usize,
    tempo: f32,
}

impl<T> NbsAudioRenderer<T>
where
    T: NoteStream,
{
    pub fn new(
        note_stream: T,
        audio_provider: impl InstrumentAudioProvider + Send + 'static,
        sample_rate: SampleRate,
        params: NbsAudioRendererParams,
    ) -> Self {
        let tempo = note_stream.default_tempo();
        let prefetch_note_stream = note_stream.clone();
        let note_stream = Some(note_stream);
        let audio_provider = Box::new(audio_provider);
        let audio_provider = NoteAudioProvider::new(
            sample_rate,
            params.num_threads,
            params.cache_capacity,
            params.interpolation_type,
            audio_provider,
        );
        let NbsAudioRendererParams {
            miss_policy,
            prefetchable_cap,
            ..
        } = params;
        NbsAudioRenderer {
            note_stream,
            audio_provider,
            prefetchable_cap,
            prefetch_note_stream,
            sample_rate,
            miss_policy,
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
        if let Some(prefetch_note_stream) = &mut self.prefetch_note_stream {
            while self.audio_provider.prefetched_count() < self.prefetchable_cap.get() {
                match prefetch_note_stream.next_event() {
                    NoteStreamEvent::NotePlay { note, weight } => {
                        self.audio_provider.prefetch(note, weight);
                    }
                    NoteStreamEvent::EndOfStream => {
                        self.prefetch_note_stream = None;
                        break;
                    }
                    _ => {}
                }
            }
        }
        while let Some(note_stream) = &mut self.note_stream {
            match note_stream.next_event() {
                NoteStreamEvent::NotePlay { note, weight } => {
                    let audio = self.audio_provider.get(note, weight, self.miss_policy);
                    if let Some(audio) = audio {
                        self.mixer.mix_note(audio);
                    }
                }
                NoteStreamEvent::TempoChange(tempo) => self.tempo = tempo,
                NoteStreamEvent::TickAdvance => break,
                NoteStreamEvent::EndOfStream => {
                    self.note_stream = None;
                    break;
                }
            }
        }
    }
}

impl<T> Iterator for NbsAudioRenderer<T>
where
    T: NoteStream,
{
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
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
}

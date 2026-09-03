use std::{num::NonZeroUsize, thread};

use crate::{
    audio::{
        Frame, NoteAudioMissPolicy, NoteAudioProvider, NoteStream, NoteStreamEvent, SampleRate,
        mixer::NoteAudioMixer,
        provider::{InstrumentAudioProvider, VanillaAudioProvider},
        resampler::InterpolationType,
    },
    instrument::InstrumentSet,
};

pub struct NbsAudioRenderer<T>
where
    T: NoteStream,
{
    note_stream: Option<T>,
    sample_rate: SampleRate,
    miss_policy: NoteAudioMissPolicy,
    audio_provider: NoteAudioProvider,
    prefetchable_cap: NonZeroUsize,
    prefetch_note_stream: Option<T>,
    samples_until_next_tick: usize,
    tempo: f32,
    mixer: NoteAudioMixer,
}

impl<T> NbsAudioRenderer<T>
where
    T: NoteStream,
{
    pub fn builder(
        note_stream: T,
        instrument_set: &InstrumentSet,
        sample_rate: SampleRate,
    ) -> NbsAudioRendererBuilder<T> {
        NbsAudioRendererBuilder::new(note_stream, instrument_set, sample_rate)
    }

    pub fn new(note_stream: T, instrument_set: &InstrumentSet, sample_rate: SampleRate) -> Self {
        NbsAudioRendererBuilder::new(note_stream, instrument_set, sample_rate).build()
    }

    pub fn with_audio_provider(
        note_stream: T,
        instrument_set: &InstrumentSet,
        sample_rate: SampleRate,
        audio_provider: impl InstrumentAudioProvider + Send + 'static,
    ) -> Self {
        NbsAudioRendererBuilder::new(note_stream, instrument_set, sample_rate)
            .audio_provider(audio_provider)
            .build()
    }

    fn new_inner(
        note_stream: T,
        num_threads: NonZeroUsize,
        audio_provider: Box<dyn InstrumentAudioProvider + Send>,
        prefetchable_cap: NonZeroUsize,
        cache_capacity: Option<NonZeroUsize>,
        sample_rate: SampleRate,
        interpolation_type: InterpolationType,
        miss_policy: NoteAudioMissPolicy,
    ) -> Self {
        let tempo = note_stream.default_tempo();
        let prefetch_note_stream = note_stream.clone();
        let audio_provider = NoteAudioProvider::new(
            num_threads,
            sample_rate,
            interpolation_type,
            cache_capacity,
            audio_provider,
        );
        let note_stream = Some(note_stream);
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

pub struct NbsAudioRendererBuilder<T>
where
    T: NoteStream,
{
    note_stream: T,
    num_threads: NonZeroUsize,
    miss_policy: NoteAudioMissPolicy,
    audio_provider: Box<dyn InstrumentAudioProvider + Send>,
    prefetchable_cap: NonZeroUsize,
    cache_capacity: Option<NonZeroUsize>,
    sample_rate: SampleRate,
    interpolation_type: InterpolationType,
}

impl<T> NbsAudioRendererBuilder<T>
where
    T: NoteStream,
{
    pub fn new(note_stream: T, instrument_set: &InstrumentSet, sample_rate: SampleRate) -> Self {
        NbsAudioRendererBuilder {
            audio_provider: Box::new(VanillaAudioProvider::new(
                instrument_set.vanilla_instrument_count(),
            )),
            prefetchable_cap: 256.try_into().unwrap(),
            note_stream,
            cache_capacity: Some(NonZeroUsize::new(256).unwrap()),
            miss_policy: NoteAudioMissPolicy::SyncFallback,
            sample_rate,
            interpolation_type: InterpolationType::Cubic,
            num_threads: thread::available_parallelism().unwrap_or(1.try_into().unwrap()),
        }
    }

    pub fn num_threads(mut self, num_threads: NonZeroUsize) -> Self {
        self.num_threads = num_threads;
        self
    }

    pub fn miss_policy(mut self, policy: NoteAudioMissPolicy) -> Self {
        self.miss_policy = policy;
        self
    }

    pub fn interpolation_type(mut self, interpolation_type: InterpolationType) -> Self {
        self.interpolation_type = interpolation_type;
        self
    }

    pub fn cache_capacity(mut self, capacity: NonZeroUsize) -> Self {
        self.cache_capacity = Some(capacity);
        self
    }

    pub fn cache_unbounded(mut self) -> Self {
        self.cache_capacity = None;
        self
    }

    pub fn prefetchable_capacity(mut self, capacity: NonZeroUsize) -> Self {
        self.prefetchable_cap = capacity;
        self
    }

    pub fn audio_provider(
        mut self,
        audio_provider: impl InstrumentAudioProvider + Send + 'static,
    ) -> Self {
        self.audio_provider =
            Box::new(audio_provider) as Box<dyn InstrumentAudioProvider + Send + 'static>;
        self
    }

    pub fn build(self) -> NbsAudioRenderer<T> {
        NbsAudioRenderer::new_inner(
            self.note_stream,
            self.num_threads,
            self.audio_provider,
            self.prefetchable_cap,
            self.cache_capacity,
            self.sample_rate,
            self.interpolation_type,
            self.miss_policy,
        )
    }
}

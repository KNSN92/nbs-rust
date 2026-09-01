use std::{borrow::Borrow, num::NonZeroUsize, thread};

use crate::{
    Nbs, Tick,
    audio::{
        Frame, NoteAudioMissPolicy, NoteAudioProvider, SampleRate,
        mixer::NoteAudioMixer,
        provider::{InstrumentAudioProvider, VanillaAudioProvider},
        resampler::InterpolationType,
        tempo::TempoMap,
    },
};

pub struct NbsAudioRenderer<P>
where
    P: Borrow<Nbs>,
{
    nbs: P,
    sample_rate: SampleRate,
    miss_policy: NoteAudioMissPolicy,
    audio_provider: NoteAudioProvider,
    prefetchable_cap: NonZeroUsize,
    tick: Tick,
    prefetch_tick: Tick,
    samples_until_next_tick: usize,
    loop_count: u8,
    tempo_map: TempoMap,
    mixer: NoteAudioMixer,
}

impl<P> NbsAudioRenderer<P>
where
    P: Borrow<Nbs>,
{
    pub fn builder(nbs: P, sample_rate: SampleRate) -> NbsAudioRendererBuilder<P> {
        NbsAudioRendererBuilder::new(nbs, sample_rate)
    }

    pub fn new(nbs: P, sample_rate: SampleRate) -> Self {
        NbsAudioRendererBuilder::new(nbs, sample_rate).build()
    }

    pub fn with_audio_provider(
        nbs: P,
        sample_rate: SampleRate,
        audio_provider: impl InstrumentAudioProvider + Send + 'static,
    ) -> Self {
        NbsAudioRendererBuilder::new(nbs, sample_rate)
            .audio_provider(audio_provider)
            .build()
    }

    fn new_inner(
        nbs: P,
        num_threads: NonZeroUsize,
        audio_provider: Box<dyn InstrumentAudioProvider + Send>,
        prefetchable_cap: NonZeroUsize,
        cache_capacity: Option<NonZeroUsize>,
        sample_rate: SampleRate,
        interpolation_type: InterpolationType,
        miss_policy: NoteAudioMissPolicy,
    ) -> Self {
        let tempo_map = TempoMap::from_nbs(nbs.borrow());
        let audio_provider = NoteAudioProvider::new(
            num_threads,
            sample_rate,
            interpolation_type,
            cache_capacity,
            audio_provider,
        );
        NbsAudioRenderer {
            nbs,
            audio_provider,
            prefetchable_cap,
            sample_rate,
            miss_policy,
            tick: 0,
            prefetch_tick: 0,
            samples_until_next_tick: 0,
            loop_count: 0,
            tempo_map,
            mixer: NoteAudioMixer::new(),
        }
    }

    #[inline]
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    pub fn current_tick(&self) -> Tick {
        self.tick
    }

    pub fn current_tempo(&self) -> f32 {
        self.tempo_map.get_tempo_at(self.tick)
    }

    pub fn playing_sounds_count(&self) -> usize {
        self.mixer.mixed_notes()
    }

    //TODO: 曲の長さをDurationで取得する関数を追加したい。

    pub fn seek_to_tick(&mut self, tick: Tick) {
        self.tick = tick;
        self.prefetch_tick = tick;
        self.samples_until_next_tick = 0;
    }

    fn samples_per_tick(&self) -> usize {
        let tempo = self.tempo_map.get_tempo_at(self.tick);
        (self.sample_rate().get() as f32 / tempo).round() as usize
    }

    fn loop_if_needed(&mut self) -> bool {
        let looping = &self.nbs.borrow().header.song_meta.looping;
        if looping.enabled {
            match looping.count {
                Some(count) if self.loop_count < count.get() => {
                    self.loop_count += 1;
                    self.seek_to_tick(looping.start_tick as u32);
                }
                Some(_) if self.mixer.is_empty() => return false,
                Some(_) => {}
                None => self.seek_to_tick(looping.start_tick as u32),
            }
        } else if self.mixer.is_empty() {
            return false;
        }
        true
    }

    fn tick(&mut self) {
        while self.audio_provider.prefetched_count() < self.prefetchable_cap.get()
            && self.prefetch_tick < self.nbs.borrow().note_blocks.ticks_len()
        {
            if let Some(notes_in_tick) = self
                .nbs
                .borrow()
                .note_blocks
                .notes_at_tick(self.prefetch_tick)
            {
                for (_, note) in notes_in_tick {
                    let custom_instrument = self
                        .nbs
                        .borrow()
                        .instrument_set
                        .custom_instrument(note.instrument);
                    self.audio_provider.prefetch(*note, custom_instrument);
                }
            }
            self.prefetch_tick += 1;
        }
        if let Some(notes_in_tick) = self
            .nbs
            .borrow()
            .note_blocks
            .notes_at_tick(self.tick)
            .cloned()
        {
            for (layer, note) in notes_in_tick {
                let layer = self.nbs.borrow().note_blocks.layer(layer);
                let custom_instrument = self
                    .nbs
                    .borrow()
                    .instrument_set
                    .custom_instrument(note.instrument);
                let audio =
                    self.audio_provider
                        .get(note, layer, custom_instrument, self.miss_policy);
                if let Some(audio) = audio {
                    self.mixer.mix_note(audio);
                }
            }
        }
        self.tick += 1;
    }
}

impl<P> Iterator for NbsAudioRenderer<P>
where
    P: Borrow<Nbs>,
{
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_until_next_tick == 0 {
            if self.tick >= self.nbs.borrow().note_blocks.ticks_len() && !self.loop_if_needed() {
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

pub struct NbsAudioRendererBuilder<P>
where
    P: Borrow<Nbs>,
{
    nbs: P,
    num_threads: NonZeroUsize,
    miss_policy: NoteAudioMissPolicy,
    audio_provider: Box<dyn InstrumentAudioProvider + Send>,
    prefetchable_cap: NonZeroUsize,
    cache_capacity: Option<NonZeroUsize>,
    sample_rate: SampleRate,
    interpolation_type: InterpolationType,
}

impl<P> NbsAudioRendererBuilder<P>
where
    P: Borrow<Nbs>,
{
    pub fn new(nbs: P, sample_rate: SampleRate) -> Self {
        NbsAudioRendererBuilder {
            audio_provider: Box::new(VanillaAudioProvider::new(
                nbs.borrow().instrument_set.vanilla_instrument_count(),
            )),
            prefetchable_cap: 256.try_into().unwrap(),
            nbs,
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

    pub fn build(self) -> NbsAudioRenderer<P> {
        NbsAudioRenderer::new_inner(
            self.nbs,
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

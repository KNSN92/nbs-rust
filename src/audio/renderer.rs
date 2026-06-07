use std::{num::NonZeroUsize, thread, time::Duration};

use crate::{
    Nbs, Tick,
    audio::{
        Frame, NoteAudio, NoteAudioMissPolicy, NoteAudioProvider, SampleRate,
        provider::{InstrumentAudioProvider, VanillaAudioProvider},
    },
};

pub struct NbsAudioRenderer {
    nbs: Nbs,
    sample_rate: SampleRate,
    miss_policy: NoteAudioMissPolicy,
    audio_provider: NoteAudioProvider,
    tick: Tick,
    prefetch_tick: Tick,
    samples_until_next_tick: usize,
    loop_count: u8,
    current_tempo_index: usize,
    tempo_mapping: Vec<(Tick, f32)>,
    playing_sounds: Vec<NoteAudio>,
}

fn build_tempo_mapping(nbs: &Nbs) -> Vec<(Tick, f32)> {
    let default_tempo = nbs.header.song_meta.tempo;
    let mut tempo_mapping = Vec::new();
    for (&tick, notes_in_tick) in nbs.note_blocks.inner_tick_notes() {
        let mut tempo = None;
        for (_, note) in notes_in_tick {
            if nbs.instrument_set.is_tempo_changer(note.instrument) {
                tempo = Some(note.pitch as f32 / 15.0);
                break;
            }
        }
        if let Some(tempo) = tempo {
            tempo_mapping.push((tick, tempo));
        }
    }
    tempo_mapping.sort_by_key(|(tick, _)| *tick);
    match tempo_mapping.first() {
        Some((tick, _)) if *tick == 0 => {}
        _ => tempo_mapping.insert(0, (0, default_tempo)),
    }
    tempo_mapping.push((
        Tick::MAX,
        tempo_mapping
            .last()
            .map(|(_, t)| *t)
            .unwrap_or(default_tempo),
    ));
    tempo_mapping
}

impl NbsAudioRenderer {
    pub fn builder(nbs: Nbs, sample_rate: SampleRate) -> NbsAudioRendererBuilder {
        NbsAudioRendererBuilder::new(nbs, sample_rate)
    }

    pub fn new(nbs: Nbs, sample_rate: SampleRate) -> Self {
        NbsAudioRendererBuilder::new(nbs, sample_rate).build()
    }

    fn new_inner(
        nbs: Nbs,
        num_threads: NonZeroUsize,
        audio_provider: Box<dyn InstrumentAudioProvider + Send>,
        cache_capacity: Option<NonZeroUsize>,
        sample_rate: SampleRate,
        miss_policy: NoteAudioMissPolicy,
    ) -> Self {
        let tempo_mapping = build_tempo_mapping(&nbs);
        let audio_provider =
            NoteAudioProvider::new(num_threads, sample_rate, cache_capacity, audio_provider);
        NbsAudioRenderer {
            nbs,
            audio_provider,
            sample_rate,
            miss_policy,
            tick: 0,
            prefetch_tick: 0,
            samples_until_next_tick: 0,
            loop_count: 0,
            current_tempo_index: 0,
            tempo_mapping,
            playing_sounds: Vec::new(),
        }
    }

    #[inline]
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    pub fn duration(&self) -> Duration {
        let total_ticks = self.nbs.note_blocks.ticks_len();
        let mut duration_secs = 0.0;
        for i in 0..self.tempo_mapping.len() - 1 {
            let (start_tick, tempo) = self.tempo_mapping[i];
            let (end_tick, _) = self.tempo_mapping[i + 1];
            if start_tick >= total_ticks {
                break;
            }
            let ticks_in_segment = (end_tick.min(total_ticks) - start_tick) as f32;
            duration_secs += ticks_in_segment / tempo;
        }
        //TODO: The duration calculation is currently based on the tempo changes and ticks, which may not be accurate if there are long audio samples that sustain over multiple ticks. We need to consider the duration of each note and its pitch to calculate a more accurate duration of the song.
        // let longest_instrument_duration: Duration = (0..self.nbs.instrument_set.instrument_count())
        //     .map(Instrument)
        //     .filter_map(|ins| self.audio_provider.get_audio(ins).map(|audio| (ins, audio)))
        //     .map(|(ins, audio)| {
        //         audio.duration().as_secs_f64() / pitch(
        //             &Note {
        //                 instrument: ins,
        //                 key: 0,
        //                 ..Default::default()
        //             },
        //             self.nbs.instrument_set.custom_instrument(ins)
        //         )
        //     })
        //     .map(Duration::from_secs_f64)
        //     .max()
        //     .unwrap_or(Duration::ZERO);
        // let fastest_tempo = self.tempo_mapping.iter().map(|(_, t)| *t as f64).fold(f64::NEG_INFINITY, f64::max);
        // let upstreaming_ticks = (longest_instrument_duration.as_secs_f64() * fastest_tempo).ceil() as u32;
        // let mut duration_secs = f32::NEG_INFINITY;
        // for tick in self.nbs.note_blocks.ticks().iter().rev().map(|t| *t) {
        //     if tick + upstreaming_ticks < total_ticks {
        //         break;
        //     }
        //     duration_secs = duration_secs.max();
        // }
        Duration::from_secs_f32(duration_secs)
    }

    pub fn duration_including_loop(&self) -> Option<Duration> {
        let (loop_count, loop_start_tick) = if self.nbs.header.song_meta.looping.enabled
            && let Some(loop_count) = self.nbs.header.song_meta.looping.count
        {
            (
                loop_count.get() as u32,
                self.nbs.header.song_meta.looping.start_tick as u32,
            )
        } else {
            return None;
        };
        let mut first_duration_secs = 0.0;
        for i in 0..self.tempo_mapping.len() {
            let (start_tick, tempo) = self.tempo_mapping[i];
            let (end_tick, _) = self.tempo_mapping[i + 1];
            if start_tick >= loop_start_tick {
                break;
            }
            let ticks_in_segment = (end_tick.min(loop_start_tick) - start_tick) as f32;
            first_duration_secs += ticks_in_segment / tempo;
        }
        let first_duration = Duration::from_secs_f32(first_duration_secs);
        let loop_duration = self.duration() - first_duration;
        let total_duration = first_duration + loop_duration * loop_count;
        Some(total_duration)
    }

    pub fn seek_to_tick(&mut self, tick: Tick) {
        self.tick = tick;
        self.prefetch_tick = tick;
        self.samples_until_next_tick = 0;
        self.current_tempo_index = self
            .tempo_mapping
            .binary_search_by(|(t, _)| t.cmp(&tick))
            .unwrap_or_else(|e| e);
    }

    fn samples_per_tick(&self) -> usize {
        let tempo = self.tempo_mapping[self.current_tempo_index].1;
        (self.sample_rate().get() as f32 / tempo).round() as usize
    }

    fn loop_if_needed(&mut self) -> bool {
        let looping = &self.nbs.header.song_meta.looping;
        if looping.enabled {
            match looping.count {
                Some(count) if self.loop_count < count.get() => {
                    self.loop_count += 1;
                    self.seek_to_tick(looping.start_tick as u32);
                }
                Some(_) if self.playing_sounds.is_empty() => return false,
                Some(_) => {}
                None => self.seek_to_tick(looping.start_tick as u32),
            }
        } else if self.playing_sounds.is_empty() {
            return false;
        }
        true
    }

    fn tick(&mut self) {
        if self.tick >= self.tempo_mapping[self.current_tempo_index + 1].0 {
            self.current_tempo_index += 1;
        }
        while self.audio_provider.prefetched_count() < 256
            && self.prefetch_tick < self.nbs.note_blocks.ticks_len()
        {
            if let Some(notes_in_tick) = self.nbs.note_blocks.notes_at_tick(self.prefetch_tick) {
                for (_, note) in notes_in_tick {
                    let custom_instrument =
                        self.nbs.instrument_set.custom_instrument(note.instrument);
                    self.audio_provider.prefetch(*note, custom_instrument);
                }
            }
            self.prefetch_tick += 1;
        }
        if let Some(notes_in_tick) = self.nbs.note_blocks.notes_at_tick(self.tick).cloned() {
            for (layer, note) in notes_in_tick {
                let layer = self.nbs.note_blocks.layer(layer);
                let custom_instrument = self.nbs.instrument_set.custom_instrument(note.instrument);
                let audio =
                    self.audio_provider
                        .get(note, layer, custom_instrument, self.miss_policy);
                if let Some(audio) = audio {
                    self.playing_sounds.push(audio);
                }
            }
        }
        self.tick += 1;
    }
}

impl Iterator for NbsAudioRenderer {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_until_next_tick == 0 {
            if self.tick >= self.nbs.note_blocks.ticks_len() && !self.loop_if_needed() {
                return None;
            }
            self.tick();
            self.samples_until_next_tick = self.samples_per_tick();
        } else {
            self.samples_until_next_tick -= 1;
        }
        let mut frame = [0.0; 2];
        self.playing_sounds.retain_mut(|sound| {
            if let Some(s) = sound.next() {
                frame[0] += s[0];
                frame[1] += s[1];
                true
            } else {
                false
            }
        });
        Some(frame)
    }
}

pub struct NbsAudioRendererBuilder {
    nbs: Nbs,
    num_threads: NonZeroUsize,
    miss_policy: NoteAudioMissPolicy,
    audio_provider: Box<dyn InstrumentAudioProvider + Send>,
    cache_capacity: Option<NonZeroUsize>,
    sample_rate: SampleRate,
}

impl NbsAudioRendererBuilder {
    pub fn new(nbs: Nbs, sample_rate: SampleRate) -> Self {
        NbsAudioRendererBuilder {
            audio_provider: Box::new(VanillaAudioProvider::new(
                nbs.instrument_set.vanilla_instrument_count(),
            )),
            nbs,
            cache_capacity: Some(NonZeroUsize::new(256).unwrap()),
            miss_policy: NoteAudioMissPolicy::SyncFallback,
            sample_rate,
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

    pub fn cache_capacity(mut self, capacity: NonZeroUsize) -> Self {
        self.cache_capacity = Some(capacity);
        self
    }

    pub fn cache_unbounded(mut self) -> Self {
        self.cache_capacity = None;
        self
    }

    pub fn sample_rate(mut self, sample_rate: SampleRate) -> Self {
        self.sample_rate = sample_rate;
        self
    }

    pub fn audio_provider(
        self,
        audio_provider: impl InstrumentAudioProvider + Send + 'static,
    ) -> NbsAudioRendererBuilder {
        let NbsAudioRendererBuilder {
            nbs,
            cache_capacity,
            miss_policy,
            sample_rate,
            audio_provider: _,
            num_threads,
        } = self;
        let audio_provider =
            Box::new(audio_provider) as Box<dyn InstrumentAudioProvider + Send + 'static>;
        NbsAudioRendererBuilder {
            nbs,
            num_threads,
            cache_capacity,
            miss_policy,
            sample_rate,
            audio_provider,
        }
    }

    pub fn build(self) -> NbsAudioRenderer {
        NbsAudioRenderer::new_inner(
            self.nbs,
            self.num_threads,
            self.audio_provider,
            self.cache_capacity,
            self.sample_rate,
            self.miss_policy,
        )
    }
}

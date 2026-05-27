use std::{num::NonZeroUsize, time::Duration};

use lru::LruCache;

use crate::{
    Nbs, Tick,
    audio::{
        Frame, NoteAudio, SampleRate,
        provider::{InstrumentAudioProvider, VanillaAudioProvider},
    },
    instrument::Instrument,
    noteblock::{LayerId, Note},
};

pub struct NbsAudioRenderer {
    nbs: Nbs,
    audio_provider: Box<dyn InstrumentAudioProvider + Send>,
    note_cache: LruCache<(Instrument, u8, i16), NoteAudio>,
    sample_rate: SampleRate,
    tick: Tick,
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
    pub fn builder(nbs: Nbs) -> NbsAudioRendererBuilder {
        NbsAudioRendererBuilder {
            audio_provider: Box::new(VanillaAudioProvider::new(
                nbs.instrument_set.vanilla_instrument_count(),
            )),
            nbs,
            cache_capacity: Some(NonZeroUsize::new(256).unwrap()),
            sample_rate: 48000u32.try_into().unwrap(),
        }
    }

    fn new(
        nbs: Nbs,
        audio_provider: Box<dyn InstrumentAudioProvider + Send>,
        cache_capacity: Option<NonZeroUsize>,
        sample_rate: SampleRate,
    ) -> Self {
        let tempo_mapping = build_tempo_mapping(&nbs);
        let note_cache = if let Some(capacity) = cache_capacity {
            LruCache::new(capacity)
        } else {
            LruCache::unbounded()
        };
        NbsAudioRenderer {
            audio_provider,
            nbs,
            note_cache,
            sample_rate,
            tick: 0,
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
        for i in 0..self.tempo_mapping.len() {
            let (start_tick, tempo) = self.tempo_mapping[i];
            let (end_tick, _) = self.tempo_mapping[i + 1];
            if start_tick >= total_ticks {
                break;
            }
            let ticks_in_segment = (end_tick.min(total_ticks) - start_tick) as f32;
            duration_secs += ticks_in_segment / tempo;
        }
        Duration::from_secs_f32(duration_secs)
    }

    pub fn duration_including_loop(&self) -> Option<Duration> {
        let (loop_count, loop_start_tick) = if self.nbs.header.song_meta.looping.enabled && let Some(loop_count) = self.nbs.header.song_meta.looping.count {
            (loop_count.get() as u32, self.nbs.header.song_meta.looping.start_tick as u32)
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
        self.current_tempo_index = self
            .tempo_mapping
            .binary_search_by(|(t, _)| t.cmp(&tick))
            .unwrap_or_else(|e| e);
    }

    fn note_audio(&mut self, note: &Note, layer: LayerId) -> Option<NoteAudio> {
        let cache_key = (note.instrument, note.key, note.pitch);
        let layer = self.nbs.note_blocks.layer(layer);
        match self.note_cache.get(&cache_key) {
            Some(cached_audio) => Some(cached_audio.for_note(note, layer)),
            None => {
                let audio = NoteAudio::new(
                    note,
                    layer,
                    self.nbs.instrument_set.custom_instrument(note.instrument),
                    &*self.audio_provider,
                    self.sample_rate,
                )?;
                let playback_audio = audio.for_note(note, layer);
                self.note_cache.put(cache_key, audio);
                Some(playback_audio)
            }
        }
    }

    fn samples_per_tick(&self) -> usize {
        let tempo = self.tempo_mapping[self.current_tempo_index].1;
        (self.sample_rate().get() as f32 / tempo).round() as usize
    }
}

impl Iterator for NbsAudioRenderer {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_until_next_tick == 0 {
            if self.tick >= self.nbs.note_blocks.ticks_len() {
                let looping = &self.nbs.header.song_meta.looping;
                if looping.enabled {
                    match looping.count {
                        Some(count) if self.loop_count < count.get() => {
                            self.loop_count += 1;
                            self.seek_to_tick(looping.start_tick as u32);
                        }
                        Some(_) if self.playing_sounds.is_empty() => return None,
                        Some(_) => {}
                        None => self.seek_to_tick(looping.start_tick as u32),
                    }
                } else {
                    return None;
                }
            }

            if self.tick >= self.tempo_mapping[self.current_tempo_index + 1].0 {
                self.current_tempo_index += 1;
            }
            if let Some(notes_in_tick) = self.nbs.note_blocks.notes_at_tick(self.tick).cloned() {
                for (layer, note) in notes_in_tick {
                    if let Some(source) = self.note_audio(&note, layer) {
                        self.playing_sounds.push(source);
                    }
                }
            }
            self.tick += 1;
            self.samples_until_next_tick = self.samples_per_tick();

            // This is the legacy debugging code. If we again need to debug performance issue, We will resuscitate this code and some codes about collecting data. ;)
            // println!(
            //     "Tick: {} / {}, Playing Sounds: {:<3} (max: {:<4}), Added Sounds: {:<2}, Cache Hit Rate: {:.2}%",
            //     self.tick,
            //     self.nbs.note_blocks.ticks_len(),
            //     self.playing_sounds.len(),
            //     self.max_sound_count,
            //     self.playing_sounds.len() - prev_sounds,
            //     (self.cache_hits as f32 / (self.cache_hits + self.cache_misses) as f32) * 100.0
            // );
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
        //TODO: We need to find a best way to clipping the samplee (maybe not needed?)
        // let active = self.playing_sounds.len().max(1) as f32;
        // let sample = sample / active.sqrt();
        // let sample = sample.clamp(-1.0, 1.0);
        Some(frame)
    }
}

pub struct NbsAudioRendererBuilder {
    nbs: Nbs,
    audio_provider: Box<dyn InstrumentAudioProvider + Send>,
    cache_capacity: Option<NonZeroUsize>,
    sample_rate: SampleRate,
}

impl NbsAudioRendererBuilder {
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
            sample_rate,
            audio_provider: _,
        } = self;
        let audio_provider =
            Box::new(audio_provider) as Box<dyn InstrumentAudioProvider + Send + 'static>;
        NbsAudioRendererBuilder {
            nbs,
            cache_capacity,
            sample_rate,
            audio_provider,
        }
    }

    pub fn build(self) -> NbsAudioRenderer {
        NbsAudioRenderer::new(
            self.nbs,
            self.audio_provider,
            self.cache_capacity,
            self.sample_rate,
        )
    }
}

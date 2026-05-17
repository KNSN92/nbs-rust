use std::num::NonZeroUsize;

use lru::LruCache;

use crate::{
    Instrument, Nbs, Tick,
    audio::{
        Frame, NoteAudio, SampleRate, VanillaAudioProvider, provider::InstrumentAudioProvider,
    },
    nbs::{LayerId, Note},
};

pub struct NbsAudioRenderer<P: InstrumentAudioProvider + Send> {
    nbs: Nbs,
    audio_provider: P,
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
        let mut tempo = (tick == 0).then_some(default_tempo);
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
    // Add a tempo mapping at the end of the song to simplify tempo changing logic.
    tempo_mapping.push((
        Tick::MAX,
        tempo_mapping
            .last()
            .map(|(_, t)| *t)
            .unwrap_or(default_tempo),
    ));
    tempo_mapping
}

impl NbsAudioRenderer<VanillaAudioProvider> {
    pub fn builder(nbs: Nbs) -> NbsAudioRendererBuilder<VanillaAudioProvider> {
        NbsAudioRendererBuilder {
            audio_provider: VanillaAudioProvider::new(
                nbs.instrument_set.vanilla_instrument_count(),
            ),
            nbs,
            cache_capacity: None,
            sample_rate: None,
        }
    }
}

impl<P: InstrumentAudioProvider + Send> NbsAudioRenderer<P> {
    fn new(
        nbs: Nbs,
        audio_provider: P,
        cache_capacity: NonZeroUsize,
        sample_rate: SampleRate,
    ) -> Self {
        let tempo_mapping = build_tempo_mapping(&nbs);
        NbsAudioRenderer {
            audio_provider,
            nbs,
            note_cache: LruCache::new(cache_capacity),
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
                    self.nbs.instrument_set.get(note.instrument),
                    &self.audio_provider,
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

impl<P: InstrumentAudioProvider + Send> Iterator for NbsAudioRenderer<P> {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.samples_until_next_tick == 0 {
            if self.tick > self.nbs.note_blocks.ticks_len() {
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

            if self.tick >= self.tempo_mapping[self.current_tempo_index].0 {
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

pub struct NbsAudioRendererBuilder<P: InstrumentAudioProvider + Send> {
    nbs: Nbs,
    audio_provider: P,
    cache_capacity: Option<NonZeroUsize>,
    sample_rate: Option<SampleRate>,
}

impl<P: InstrumentAudioProvider + Send> NbsAudioRendererBuilder<P> {
    pub fn cache_capacity(mut self, capacity: NonZeroUsize) -> Self {
        self.cache_capacity = Some(capacity);
        self
    }

    pub fn sample_rate(mut self, sample_rate: SampleRate) -> Self {
        self.sample_rate = Some(sample_rate);
        self
    }

    pub fn audio_provider<NP: InstrumentAudioProvider + Send>(
        self,
        audio_provider: NP,
    ) -> NbsAudioRendererBuilder<NP> {
        let NbsAudioRendererBuilder {
            nbs,
            cache_capacity,
            sample_rate,
            audio_provider: _,
        } = self;
        NbsAudioRendererBuilder {
            nbs,
            cache_capacity,
            sample_rate,
            audio_provider,
        }
    }

    pub fn build(self) -> NbsAudioRenderer<P> {
        NbsAudioRenderer::new(
            self.nbs,
            self.audio_provider,
            self.cache_capacity.unwrap_or(256usize.try_into().unwrap()),
            self.sample_rate.unwrap_or(48000u32.try_into().unwrap()),
        )
    }
}

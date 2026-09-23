use std::{
    collections::VecDeque,
    num::NonZeroUsize,
    sync::{Arc, OnceLock},
};

use lru::LruCache;

use crate::{
    audio::{
        AudioBuffer, NbsEvent, NbsStream, SampleRate,
        instrument::InstrumentAudioProvider,
        note_audio::{NoteAudio, NoteAudioKey, NoteWeight},
        resampler::{
            AsyncAudioResampler, SyncAudioResampler,
            multithreaded::{MultithreadedResampler, NumThreads},
            polynomial::{InterpolationType, PolynomialResampler},
        },
    },
    noteblock::Note,
};

pub struct PrefetchableNoteAudioStreamParams<R: SyncAudioResampler + Send + 'static> {
    pub new_resampler: fn() -> R,
    pub num_threads: NumThreads,
    pub cache_capacity: CacheCapacity,
    pub prefetch_miss_policy: NoteAudioPrefetchMissPolicy,
    pub prefetchable_event_capacity: NonZeroUsize,
}

impl Default for PrefetchableNoteAudioStreamParams<PolynomialResampler> {
    fn default() -> Self {
        PrefetchableNoteAudioStreamParams {
            new_resampler: || PolynomialResampler::new(InterpolationType::Cubic),
            num_threads: NumThreads::default(),
            cache_capacity: CacheCapacity::Bounded(256.try_into().unwrap()),
            prefetch_miss_policy: NoteAudioPrefetchMissPolicy::SyncFallback(Box::new(
                PolynomialResampler::new(InterpolationType::Cubic),
            )),
            prefetchable_event_capacity: 256.try_into().unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CacheCapacity {
    Bounded(NonZeroUsize),
    Unbounded,
}

pub struct PrefetchableNoteAudioStream<S, C, I>
where
    S: NbsStream<(Note, NoteWeight), C>,
    C: Clone,
    I: InstrumentAudioProvider,
{
    note_stream: S,
    instr_provider: I,
    sample_rate: SampleRate,
    resampler: MultithreadedResampler,
    prefetch_miss_policy: NoteAudioPrefetchMissPolicy,
    audio_cache: LruCache<NoteAudioKey, Arc<PrefetchedNoteAudio>>,
    prefetchable_event_capacity: NonZeroUsize,
    prefetched_events: VecDeque<NbsEvent<(Note, NoteWeight, Arc<PrefetchedNoteAudio>), C>>,
}

pub enum NoteAudioPrefetchMissPolicy {
    SyncFallback(Box<dyn SyncAudioResampler + Send + 'static>),
    Wait,
    Skip,
}

struct PrefetchedNoteAudio(OnceLock<Option<AudioBuffer>>);

impl PrefetchedNoteAudio {
    pub fn new_ready(audio: AudioBuffer) -> Self {
        PrefetchedNoteAudio(OnceLock::from(Some(audio)))
    }

    pub fn new_not_ready() -> Self {
        PrefetchedNoteAudio(OnceLock::new())
    }

    pub fn new_failed() -> Self {
        PrefetchedNoteAudio(OnceLock::from(None))
    }

    pub fn get(&self) -> PrefetchedNoteAudioValueWithState {
        let audio = self.0.get().cloned();
        match audio {
            Some(Some(audio)) => PrefetchedNoteAudioValueWithState::Ready(audio),
            Some(None) => PrefetchedNoteAudioValueWithState::Failed,
            None => PrefetchedNoteAudioValueWithState::NotReady,
        }
    }

    pub fn set(&self, audio: Option<AudioBuffer>) -> Result<(), ()> {
        self.0.set(audio).map_err(|_| ())
    }

    pub fn wait(&self) -> PrefetchedNoteAudioValueWithState {
        let audio = self.0.wait().clone();
        match audio {
            Some(audio) => PrefetchedNoteAudioValueWithState::Ready(audio),
            None => PrefetchedNoteAudioValueWithState::Failed,
        }
    }
}

enum PrefetchedNoteAudioValueWithState {
    Ready(AudioBuffer),
    NotReady,
    Failed,
}

impl<S, C, I> PrefetchableNoteAudioStream<S, C, I>
where
    S: NbsStream<(Note, NoteWeight), C>,
    C: Clone,
    I: InstrumentAudioProvider,
{
    pub fn new(
        note_stream: S,
        instr_provider: I,
        sample_rate: SampleRate,
        params: PrefetchableNoteAudioStreamParams<impl SyncAudioResampler + Send + 'static>,
    ) -> Self {
        let PrefetchableNoteAudioStreamParams {
            new_resampler,
            num_threads,
            cache_capacity,
            prefetch_miss_policy,
            prefetchable_event_capacity,
        } = params;
        let audio_cache = match cache_capacity {
            CacheCapacity::Bounded(cap) => LruCache::new(cap),
            CacheCapacity::Unbounded => LruCache::unbounded(),
        };
        let resampler = MultithreadedResampler::new(num_threads, new_resampler);
        PrefetchableNoteAudioStream {
            note_stream,
            instr_provider,
            sample_rate,
            resampler,
            prefetch_miss_policy,
            audio_cache,
            prefetched_events: VecDeque::new(),
            prefetchable_event_capacity,
        }
    }
}

impl<S, C, I> NbsStream<NoteAudio, C> for PrefetchableNoteAudioStream<S, C, I>
where
    S: NbsStream<(Note, NoteWeight), C>,
    C: Clone,
    I: InstrumentAudioProvider,
{
    fn next_event(&mut self) -> NbsEvent<NoteAudio, C> {
        while self.prefetched_events.len() < self.prefetchable_event_capacity.get() {
            match self.note_stream.next_event() {
                NbsEvent::NotePlay((note, weight)) => {
                    let audio = if let Some(audio_lock) =
                        self.audio_cache.get(&NoteAudioKey::from(note)).cloned()
                    {
                        audio_lock
                    } else if let Some(audio) = self.instr_provider.get_audio(note.instrument) {
                        let audio_lock = PrefetchedNoteAudio::new_not_ready().into();
                        let pitch = note.pitch(weight);
                        let audio_weak_lock = Arc::downgrade(&audio_lock);
                        self.resampler.request_resample(
                            audio,
                            self.sample_rate,
                            pitch,
                            move |audio| {
                                if let Some(audio_lock) = audio_weak_lock.upgrade() {
                                    let _ = audio_lock.set(audio);
                                }
                            },
                        );
                        self.audio_cache
                            .put(NoteAudioKey::from(note), audio_lock.clone());
                        audio_lock
                    } else {
                        PrefetchedNoteAudio::new_failed().into()
                    };
                    self.prefetched_events
                        .push_back(NbsEvent::NotePlay((note, weight, audio)));
                }
                NbsEvent::TempoChange(tempo) => {
                    self.prefetched_events
                        .push_back(NbsEvent::TempoChange(tempo));
                }
                NbsEvent::CustomEvent(c) => {
                    self.prefetched_events.push_back(NbsEvent::CustomEvent(c));
                }
                NbsEvent::NoOp => {
                    self.prefetched_events.push_back(NbsEvent::NoOp);
                }
                NbsEvent::TickAdvance => {
                    self.prefetched_events.push_back(NbsEvent::TickAdvance);
                }
                NbsEvent::EndOfStream => {
                    self.prefetched_events.push_back(NbsEvent::EndOfStream);
                }
            }
        }
        let Some(event) = self.prefetched_events.pop_front() else {
            return NbsEvent::EndOfStream;
        };
        let (note, weight, audio) = match event {
            NbsEvent::NotePlay((note, weight, audio)) => match audio.get() {
                PrefetchedNoteAudioValueWithState::Ready(audio) => {
                    return NbsEvent::NotePlay(NoteAudio::new(audio.clone(), note, weight));
                }
                PrefetchedNoteAudioValueWithState::Failed => return NbsEvent::NoOp,
                PrefetchedNoteAudioValueWithState::NotReady => (note, weight, audio),
            },
            NbsEvent::TempoChange(tempo) => return NbsEvent::TempoChange(tempo),
            NbsEvent::CustomEvent(c) => return NbsEvent::CustomEvent(c),
            NbsEvent::NoOp => return NbsEvent::NoOp,
            NbsEvent::TickAdvance => return NbsEvent::TickAdvance,
            NbsEvent::EndOfStream => return NbsEvent::EndOfStream,
        };
        match self
            .audio_cache
            .get(&NoteAudioKey::from(note))
            .map(|a| a.get())
        {
            Some(PrefetchedNoteAudioValueWithState::Ready(audio)) => {
                let audio = NoteAudio::new(audio.clone(), note, weight);
                return NbsEvent::NotePlay(audio);
            }
            Some(PrefetchedNoteAudioValueWithState::Failed) => {
                self.audio_cache.pop(&NoteAudioKey::from(note));
                return NbsEvent::NoOp;
            }
            _ => {}
        }
        match &self.prefetch_miss_policy {
            NoteAudioPrefetchMissPolicy::SyncFallback(resampler) => {
                let pitch = note.pitch(weight);
                let audio = self
                    .instr_provider
                    .get_audio(note.instrument)
                    .map(|audio| resampler.resample(audio, self.sample_rate, pitch))
                    .flatten();
                if let Some(audio) = audio {
                    self.audio_cache.put(
                        NoteAudioKey::from(note),
                        PrefetchedNoteAudio::new_ready(audio.clone()).into(),
                    );
                    let audio = NoteAudio::new(audio, note, weight);
                    NbsEvent::NotePlay(audio)
                } else {
                    NbsEvent::NoOp
                }
            }
            NoteAudioPrefetchMissPolicy::Wait => match audio.wait() {
                PrefetchedNoteAudioValueWithState::Ready(audio) => {
                    NbsEvent::NotePlay(NoteAudio::new(audio.clone(), note, weight))
                }
                _ => NbsEvent::NoOp,
            },
            NoteAudioPrefetchMissPolicy::Skip => NbsEvent::NoOp,
        }
    }

    fn default_tempo(&self) -> f32 {
        self.note_stream.default_tempo()
    }
}

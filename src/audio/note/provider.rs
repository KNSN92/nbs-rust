use std::{
    collections::{HashMap, hash_map::Entry},
    num::NonZeroUsize,
    thread,
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender, unbounded};
use lru::LruCache;

use crate::{
    audio::{
        SampleRate,
        instrument::{InstrumentAudio, InstrumentAudioProvider},
        note::{Frames, NoteAudio, NoteAudioKey, NoteWeight},
        resampler::{
            NoteAudioResampler,
            polynomial::{InterpolationType, PolynomialResampler},
        },
    },
    noteblock::Note,
};

pub struct NoteAudioProvider {
    audio_cache: LruCache<NoteAudioKey, Frames>,
    provider: Box<dyn InstrumentAudioProvider + Send>,
    sample_rate: SampleRate,
    interpolation_type: InterpolationType,

    task_tx: Option<Sender<NoteAudioResampleTask>>,
    result_rx: Receiver<NoteAudioResampleResult>,
    threads: Vec<thread::JoinHandle<()>>,

    prefetched_audios: HashMap<NoteAudioKey, (usize, NoteAudioWithState)>,
}

#[derive(Debug, Clone, Copy)]
pub enum NoteAudioMissPolicy {
    SyncFallback,
    Wait(Option<Duration>),
    Skip,
}

#[derive(Debug, Clone)]
enum NoteAudioWithState {
    Ready(Frames),
    Failed,
    Fetching,
}

struct NoteAudioResampleTask {
    key: NoteAudioKey,
    pitch: f64,
    audio: InstrumentAudio,
}

struct NoteAudioResampleResult {
    key: NoteAudioKey,
    audio: Option<Frames>,
}

enum PrefetchedAudio {
    Ready(Frames),
    Failed,
    Fetching,
    NotFound,
}

#[derive(Debug, Clone, Copy)]
pub struct NumThreads(pub NonZeroUsize);

impl Default for NumThreads {
    fn default() -> Self {
        let num_threads = thread::available_parallelism().unwrap_or_else(|_| 1.try_into().unwrap());
        NumThreads(num_threads)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CacheCapacity {
    Bounded(NonZeroUsize),
    Unbounded,
}

impl NoteAudioProvider {
    pub fn new(
        sample_rate: SampleRate,
        num_threads: NumThreads,
        cache_cap: CacheCapacity,
        interpolation_type: InterpolationType,
        provider: Box<dyn InstrumentAudioProvider + Send>,
    ) -> Self {
        let num_threads = num_threads.0.get();
        let (task_tx, task_rx) = unbounded();
        let task_tx = Some(task_tx);
        let (result_tx, result_rx) = unbounded();
        let mut threads = Vec::with_capacity(num_threads);
        for i in 0..num_threads {
            let task_rx = task_rx.clone();
            let result_tx = result_tx.clone();
            let handle = thread::Builder::new()
                .name(format!("NoteAudioResampleWorker-{}", i))
                .spawn(move || worker(task_rx, result_tx, sample_rate, interpolation_type))
                .unwrap();
            threads.push(handle);
        }
        let audio_cache = match cache_cap {
            CacheCapacity::Bounded(cap) => LruCache::new(cap),
            CacheCapacity::Unbounded => LruCache::unbounded(),
        };
        Self {
            audio_cache,
            provider,
            sample_rate,
            interpolation_type,
            task_tx,
            result_rx,
            threads,
            prefetched_audios: HashMap::new(),
        }
    }

    pub fn prefetch(&mut self, note: Note, weight: NoteWeight) {
        let key = NoteAudioKey::from(note);
        if let Some(audio) = self.audio_cache.get(&key) {
            self.prefetched_audios
                .insert(key, (1, NoteAudioWithState::Ready(audio.clone())));
            return;
        }
        if let Some((count, _)) = self.prefetched_audios.get_mut(&key) {
            *count += 1;
            return;
        }
        let Some(audio) = self.provider.get_audio(note.instrument) else {
            return;
        };
        let pitch = note.pitch(weight);
        let is_err = self
            .task_tx
            .as_ref()
            .unwrap()
            .send(NoteAudioResampleTask { key, pitch, audio })
            .is_err();
        if is_err {
            eprintln!("Failed to send note audio resample task");
            return;
        }
        self.prefetched_audios
            .insert(key, (1, NoteAudioWithState::Fetching));
    }

    pub fn prefetched_count(&self) -> usize {
        self.prefetched_audios.len()
    }

    pub fn get(
        &mut self,
        note: Note,
        weight: NoteWeight,
        policy: NoteAudioMissPolicy,
    ) -> Option<NoteAudio> {
        self.receive_results();
        let key = NoteAudioKey::from(note);
        match self.get_prefetched(key) {
            PrefetchedAudio::Ready(audio) => {
                self.audio_cache.put(key, audio.clone());
                return Some(NoteAudio::new(audio, note, weight));
            }
            PrefetchedAudio::Failed => return None,
            _ => {}
        }
        if let Some(audio) = self.audio_cache.get(&key) {
            let audio = NoteAudio::new(audio.clone(), note, weight);
            self.consume_prefetched(key);
            return Some(audio);
        }
        match policy {
            NoteAudioMissPolicy::SyncFallback => {
                self.consume_prefetched(key);
                if let Some(audio) = self.provider.get_audio(note.instrument) {
                    let pitch = note.pitch(weight);
                    let frames = PolynomialResampler::new(self.interpolation_type).resample(
                        audio,
                        self.sample_rate,
                        pitch,
                    )?;
                    self.audio_cache.put(key, frames.clone());
                    let audio = NoteAudio::new(frames, note, weight);
                    Some(audio)
                } else {
                    None
                }
            }
            NoteAudioMissPolicy::Wait(timeout) => {
                if !self.prefetched_audios.contains_key(&key) {
                    self.prefetch(note, weight);
                }
                let timeout = timeout.unwrap_or(Duration::MAX);
                let start = std::time::Instant::now();
                while !self.prefetched_audios.is_empty() {
                    let recv_key = self.receive_result_blocking(timeout - start.elapsed());
                    if let Some(recv_key) = recv_key
                        && recv_key == key
                    {
                        break;
                    }
                    if start.elapsed() >= timeout {
                        self.consume_prefetched(key);
                        return None;
                    }
                }
                let audio = match self.get_prefetched(key) {
                    PrefetchedAudio::Ready(audio) => audio,
                    PrefetchedAudio::Failed => return None,
                    _ => {
                        self.consume_prefetched(key);
                        return None;
                    }
                };
                self.audio_cache.put(key, audio.clone());
                Some(NoteAudio::new(audio, note, weight))
            }
            NoteAudioMissPolicy::Skip => {
                self.consume_prefetched(key);
                None
            }
        }
    }

    fn receive_results(&mut self) {
        while let Ok(NoteAudioResampleResult { key, audio }) = self.result_rx.try_recv() {
            if let Some(result) = self.prefetched_audios.get_mut(&key) {
                result.1 = if let Some(audio) = audio {
                    NoteAudioWithState::Ready(audio)
                } else {
                    NoteAudioWithState::Failed
                };
            }
        }
    }

    fn receive_result_blocking(&mut self, timeout: Duration) -> Option<NoteAudioKey> {
        if let Ok(NoteAudioResampleResult { key, audio }) = self.result_rx.recv_timeout(timeout) {
            if let Some(result) = self.prefetched_audios.get_mut(&key) {
                result.1 = if let Some(audio) = audio {
                    NoteAudioWithState::Ready(audio)
                } else {
                    NoteAudioWithState::Failed
                };
            }
            Some(key)
        } else {
            None
        }
    }

    fn consume_prefetched(&mut self, key: NoteAudioKey) {
        if let Entry::Occupied(mut e) = self.prefetched_audios.entry(key) {
            if e.get().0 > 1 {
                let (c, _) = e.get_mut();
                *c -= 1;
            } else {
                e.remove();
            }
        }
    }

    fn get_prefetched(&mut self, key: NoteAudioKey) -> PrefetchedAudio {
        match self.prefetched_audios.entry(key) {
            Entry::Occupied(mut e) => {
                let (c, audio) = e.get_mut();
                let audio = if *c > 1 {
                    if !matches!(*audio, NoteAudioWithState::Fetching) {
                        *c -= 1;
                    }
                    audio.clone()
                } else {
                    e.remove().1
                };
                match audio {
                    NoteAudioWithState::Ready(audio) => PrefetchedAudio::Ready(audio),
                    NoteAudioWithState::Failed => PrefetchedAudio::Failed,
                    NoteAudioWithState::Fetching => PrefetchedAudio::Fetching,
                }
            }
            _ => PrefetchedAudio::NotFound,
        }
    }
}

impl Drop for NoteAudioProvider {
    fn drop(&mut self) {
        self.task_tx.take();
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}

fn worker(
    task_rx: Receiver<NoteAudioResampleTask>,
    result_tx: Sender<NoteAudioResampleResult>,
    sample_rate: SampleRate,
    interpolation_type: InterpolationType,
) {
    loop {
        let Ok(NoteAudioResampleTask { key, pitch, audio }) = task_rx.recv() else {
            break;
        };
        let audio =
            PolynomialResampler::new(interpolation_type).resample(audio, sample_rate, pitch);
        let _ = result_tx.send(NoteAudioResampleResult { key, audio });
    }
}

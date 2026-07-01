use core::slice;
use std::{
    collections::{HashMap, hash_map::Entry}, mem, num::{NonZeroU32, NonZeroUsize}, sync::Arc, thread, time::Duration,
};

use crossbeam_channel::{Receiver, Sender, unbounded};
use lru::LruCache;
use wide::f32x16;

use crate::{
    audio::{
        Frame, InstrumentAudio, SampleRate, provider::InstrumentAudioProvider, resample::resample_audio,
    }, instrument::{CustomInstrument, Instrument}, noteblock::{Layer, Note},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NoteAudioKey {
    instrument: Instrument,
    key: u8,
    pitch: i16,
}

impl NoteAudioKey {
    pub fn new(instrument: Instrument, key: u8, pitch: i16) -> Self {
        NoteAudioKey {
            instrument,
            key,
            pitch,
        }
    }
}

impl From<Note> for NoteAudioKey {
    fn from(note: Note) -> Self {
        NoteAudioKey {
            instrument: note.instrument,
            key: note.key,
            pitch: note.pitch,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoteAudio {
    frames: Arc<[Frame]>,
    chunk: [Frame; 8],
    chunk_pos: usize,
    multiplier: f32x16,
    sample_rate: SampleRate,
    pos: usize,
}

impl NoteAudio {
    pub fn new(
        note: &Note,
        layer: Option<&Layer>,
        custom_instrument: Option<&CustomInstrument>,
        provider: &dyn InstrumentAudioProvider,
        sample_rate: SampleRate,
    ) -> Option<Self> {
        let audio = provider.get_audio(note.instrument)?;
        let pitch = pitch(note, custom_instrument);

        let frames = resample_audio(&audio, pitch, sample_rate)?;
        Some(NoteAudio {
            frames: frames.into(),
            chunk: [[0.0; 2]; 8],
            chunk_pos: 0,
            multiplier: multiplier(note, layer),
            sample_rate,
            pos: 0,
        })
    }

    pub fn from_frames(
        frames: impl Into<Arc<[Frame]>>,
        note: Note,
        layer: Option<&Layer>,
        sample_rate: SampleRate,
    ) -> Self {
        let frames = frames.into();
        NoteAudio {
            frames,
            chunk: [[0.0; 2]; 8],
            chunk_pos: 0,
            multiplier: multiplier(&note, layer),
            sample_rate,
            pos: 0,
        }
    }

    #[inline]
    pub fn sample_rate(&self) -> NonZeroU32 {
        self.sample_rate
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.frames.len() as f64 / self.sample_rate.get() as f64)
    }

    pub fn for_note(&self, note: &Note, layer: Option<&Layer>) -> Self {
        NoteAudio {
            frames: self.frames.clone(),
            chunk: [[0.0; 2]; 8],
            chunk_pos: 0,
            multiplier: multiplier(note, layer),
            sample_rate: self.sample_rate,
            pos: 0,
        }
    }

    pub(crate) fn next_chunk_simd(&mut self) -> Option<f32x16> {
        if self.pos >= self.frames.len() {
            return None;
        }
        let frames_to_take = (self.frames.len() - self.pos).min(8);
        // SAFETY: self.pos is less than self.frames.len(), and self.pos + frames_to_take is always less than or equal to self.frames.len(), so this is safe.
        let samples_slice = unsafe {
            let samples_ptr = self.frames.as_ptr().add(self.pos) as *const f32;
            let samples_to_take = frames_to_take * 2;
            slice::from_raw_parts(samples_ptr, samples_to_take)
        };
        let chunk = f32x16::from(samples_slice) * self.multiplier;
        self.pos += frames_to_take;
        self.chunk_pos = 0;
        Some(chunk)
    }

    pub fn next_chunk(&mut self) -> Option<[Frame; 8]> {
        let chunk = self.next_chunk_simd()?;
        let chunk = unsafe { mem::transmute(chunk.to_array()) };
        Some(chunk)
    }
}

fn multiplier(note: &Note, layer: Option<&Layer>) -> f32x16 {
    let volume = volume(note, layer);
    let panning = panning(note, layer);
    // Safely transmute the array of 2-element arrays into a 16-element array, since we know the size is correct.
    let multiplier: [f32; 16] = unsafe { mem::transmute([[panning[0] * volume, panning[1] * volume]; 8]) };
    f32x16::from(multiplier)
}

fn volume(note: &Note, layer: Option<&Layer>) -> f32 {
    let layer_volume = layer
        .map(|layer| layer.volume as f32 / 100.0)
        .unwrap_or(1.0);
    let note_volume = note.volume as f32 / 100.0;
    note_volume * layer_volume
}

fn panning(note: &Note, layer: Option<&Layer>) -> [f32; 2] {
    let layer_panning = layer.map(|l| l.panning as f32 / 100.0).unwrap_or(0.0);
    let note_panning = note.panning as f32 / 100.0;
    let panning = match layer_panning {
        0.0 => note_panning,
        _ => (layer_panning + note_panning) / 2.0,
    };
    [2.0 - panning, panning]
}

pub(crate) fn pitch(note: &Note, custom_instrument: Option<&CustomInstrument>) -> f64 {
    let instrument_key = custom_instrument
        .map(|ci| ci.key as f64 - 45.0)
        .unwrap_or(0.0);
    let pitch = note.pitch as f64;
    let key = note.key as f64;
    let key = key + instrument_key + pitch / 100.0;
    let key = key - 45.0;
    2.0f64.powf(key / 12.0)
}

impl Iterator for NoteAudio {
    type Item = Frame;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.frames.len() {
            return None;
        }
        if self.chunk_pos == 0 {
            self.chunk = self.next_chunk().unwrap();
            self.chunk_pos = 0;
        }
        let frame = self.chunk[self.chunk_pos];
        self.chunk_pos = (self.chunk_pos + 1) & 7;
        self.pos += 1;
        Some(frame)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.frames.len() - self.pos;
        (remaining, Some(remaining))
    }
}

pub struct NoteAudioProvider {
    audio_cache: LruCache<NoteAudioKey, Arc<[Frame]>>,
    provider: Box<dyn InstrumentAudioProvider + Send>,
    sample_rate: SampleRate,

    task_tx: Option<Sender<NoteAudioResampleTask>>,
    result_rx: Receiver<NoteAudioResampleResult>,
    threads: Vec<thread::JoinHandle<()>>,

    prefetching_count: usize,
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
    Ready(Arc<[Frame]>),
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
    audio: Option<Arc<[Frame]>>,
}

impl NoteAudioProvider {
    pub fn new(
        num_threads: NonZeroUsize,
        sample_rate: SampleRate,
        cache_cap: Option<NonZeroUsize>,
        provider: Box<dyn InstrumentAudioProvider + Send>,
    ) -> Self {
        let num_threads = num_threads.get();
        let (task_tx, task_rx) = unbounded();
        let task_tx = Some(task_tx);
        let (result_tx, result_rx) = unbounded();
        let mut threads = Vec::with_capacity(num_threads);
        for i in 0..num_threads {
            let task_rx = task_rx.clone();
            let result_tx = result_tx.clone();
            let handle = thread::Builder::new()
                .name(format!("NoteAudioResampleWorker-{}", i))
                .spawn(move || worker(task_rx, result_tx, sample_rate))
                .unwrap();
            threads.push(handle);
        }
        let audio_cache = match cache_cap {
            Some(cap) => LruCache::new(cap),
            None => LruCache::unbounded(),
        };
        Self {
            audio_cache,
            provider,
            sample_rate,
            task_tx,
            result_rx,
            threads,
            prefetching_count: 0,
            prefetched_audios: HashMap::new(),
        }
    }

    pub fn prefetch(&mut self, note: Note, custom_instrument: Option<&CustomInstrument>) {
        let key = NoteAudioKey::from(note);
        if self.prefetched_audios.contains_key(&key) {
            self.prefetched_audios.get_mut(&key).unwrap().0 += 1;
            return;
        }
        if let Some(audio) = self.audio_cache.get(&key) {
            self.prefetched_audios
                .insert(key, (1, NoteAudioWithState::Ready(audio.clone())));
            return;
        }
        let audio = if let Some(audio) = self.provider.get_audio(note.instrument) {
            audio
        } else {
            return;
        };
        let pitch = pitch(&note, custom_instrument);
        let is_ok = self
            .task_tx
            .as_ref()
            .unwrap()
            .send(NoteAudioResampleTask { key, pitch, audio })
            .is_ok();
        if is_ok {
            self.prefetching_count += 1;
        } else {
            eprintln!("Failed to send note audio resample task");
            return;
        }
        self.prefetched_audios
            .insert(key, (1, NoteAudioWithState::Fetching));
    }

    pub fn prefetched_count(&self) -> usize {
        self.prefetched_audios.len() + self.prefetching_count
    }

    pub fn get(
        &mut self,
        note: Note,
        layer: Option<&Layer>,
        custom_instrument: Option<&CustomInstrument>,
        policy: NoteAudioMissPolicy,
    ) -> Option<NoteAudio> {
        self.receive_results();
        let key = NoteAudioKey::from(note);
        match self.get_prefetched(key) {
            Some(audio) => {
                let audio = if let Some(audio) = audio {
                    self.audio_cache.put(key, audio.clone());
                    Some(NoteAudio::from_frames(audio, note, layer, self.sample_rate))
                } else {
                    None
                };
                return audio;
            }
            None => {}
        }
        if let Some(audio) = self.audio_cache.get(&key) {
            let audio = NoteAudio::from_frames(audio.clone(), note, layer, self.sample_rate);
            return Some(audio);
        }
        match policy {
            NoteAudioMissPolicy::SyncFallback => {
                self.consume_prefetched(key);
                if let Some(audio) = self.provider.get_audio(note.instrument) {
                    let pitch = pitch(&note, custom_instrument);
                    let frames: Arc<[Frame]> =
                        resample_audio(&audio, pitch, self.sample_rate)?.into();
                    self.audio_cache.put(key, frames.clone());
                    let audio = NoteAudio::from_frames(frames, note, layer, self.sample_rate);
                    return Some(audio);
                }
            }
            NoteAudioMissPolicy::Wait(timeout) => {
                if !self.prefetched_audios.contains_key(&key) {
                    self.prefetch(note, custom_instrument);
                }
                let timeout = timeout.unwrap_or(Duration::MAX);
                let start = std::time::Instant::now();
                while self.prefetching_count > 0 {
                    let recv_key = self.receive_result_blocking(timeout - start.elapsed());
                    if let Some(recv_key) = recv_key && recv_key == key {
                        break;
                    }
                    if start.elapsed() >= timeout {
                        return None;
                    }
                }
                let audio = match self.get_prefetched(key) {
                    Some(audio) => audio,
                    None => return None,
                };
                let audio = if let Some(audio) = audio {
                    self.audio_cache.put(key, audio.clone());
                    Some(NoteAudio::from_frames(audio, note, layer, self.sample_rate))
                } else {
                    None
                };
                return audio;
            }
            NoteAudioMissPolicy::Skip => {
                self.consume_prefetched(key);
                return None;
            }
        }
        None
    }

    fn receive_results(&mut self) {
        while let Ok(NoteAudioResampleResult { key, audio }) = self.result_rx.try_recv() {
            self.prefetching_count -= 1;
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
            self.prefetching_count -= 1;
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

    fn get_prefetched(&mut self, key: NoteAudioKey) -> Option<Option<Arc<[Frame]>>> {
        match self.prefetched_audios.entry(key) {
            Entry::Occupied(mut e) => {
                let audio = if e.get().0 > 1 {
                    let (c, audio) = e.get_mut();
                    *c -= 1;
                    audio.clone()
                } else {
                    e.remove().1
                };
                match audio {
                    NoteAudioWithState::Ready(audio) => Some(Some(audio)),
                    NoteAudioWithState::Failed => return Some(None),
                    NoteAudioWithState::Fetching => None,
                }
            }
            _ => None,
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
) {
    loop {
        let NoteAudioResampleTask { key, pitch, audio } = if let Ok(task) = task_rx.recv() {
            task
        } else {
            break;
        };
        let audio = resample_audio(&audio, pitch, sample_rate).map(Arc::from);
        let _ = result_tx.send(NoteAudioResampleResult { key, audio });
    }
}

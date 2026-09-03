use std::{num::NonZeroUsize, thread};

use crossbeam_channel::{Receiver, SendError, Sender, unbounded};

use crate::audio::{
    Frames, SampleRate,
    resampler::{AsyncAudioResampler, SyncAudioResampler},
};

#[derive(Debug, Clone, Copy)]
pub struct NumThreads(pub NonZeroUsize);

impl Default for NumThreads {
    fn default() -> Self {
        let num_threads = thread::available_parallelism().unwrap_or_else(|_| 1.try_into().unwrap());
        NumThreads(num_threads)
    }
}

struct NoteAudioResampleTask {
    audio: Frames,
    sample_rate: SampleRate,
    pitch: f64,
    callback: Box<dyn FnOnce(Option<Frames>) + Send + 'static>,
}

pub struct MultithreadedResampler {
    task_tx: Option<Sender<NoteAudioResampleTask>>,
    threads: Vec<thread::JoinHandle<()>>,
}

impl MultithreadedResampler {
    pub fn new<T: SyncAudioResampler + Send + 'static>(
        num_threads: NumThreads,
        new_resampler: impl Fn() -> T,
    ) -> Self {
        let num_threads = num_threads.0.get();
        let (task_tx, task_rx) = unbounded();
        let task_tx = Some(task_tx);
        let mut threads = Vec::with_capacity(num_threads);
        for i in 0..num_threads {
            let resampler = new_resampler();
            let task_rx = task_rx.clone();
            let handle = thread::Builder::new()
                .name(format!("MultithreadedResamplingWorker-{}", i))
                .spawn(move || worker(resampler, task_rx))
                .unwrap();
            threads.push(handle);
        }
        MultithreadedResampler { task_tx, threads }
    }
}

fn worker(
    resampler: impl SyncAudioResampler + Send + 'static,
    task_rx: Receiver<NoteAudioResampleTask>,
) {
    loop {
        let Ok(NoteAudioResampleTask {
            audio,
            sample_rate,
            pitch,
            callback,
        }) = task_rx.recv()
        else {
            break;
        };
        let audio = resampler.resample(audio, sample_rate, pitch);
        callback(audio);
    }
}

impl AsyncAudioResampler for MultithreadedResampler {
    fn request_resample(
        &self,
        frames: Frames,
        sample_rate: SampleRate,
        pitch: f64,
        callback: impl FnOnce(Option<Frames>) + Send + 'static,
    ) {
        if let Some(task_tx) = &self.task_tx {
            let task = NoteAudioResampleTask {
                audio: frames,
                sample_rate,
                pitch,
                callback: Box::new(callback),
            };
            let result = task_tx.send(task);
            if let Err(SendError(NoteAudioResampleTask { callback, .. })) = result {
                callback(None);
            }
        } else {
            callback(None);
        }
    }
}

impl Drop for MultithreadedResampler {
    fn drop(&mut self) {
        self.task_tx.take();
        for handle in self.threads.drain(..) {
            handle.join().expect(
                "Resampler thread panicked! This is a bug in the resampler implementation.",
            );
        }
    }
}

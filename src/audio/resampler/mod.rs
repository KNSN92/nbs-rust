pub mod multithreaded;
pub mod polynomial;

use std::thread;

use crossbeam_channel::{SendError, Sender, unbounded};

use crate::audio::{AudioBuffer, SampleRate};

pub trait SyncAudioResampler {
    fn resample(&self, frames: AudioBuffer, sample_rate: SampleRate, pitch: f64) -> Option<AudioBuffer>;

    fn into_async(self) -> impl AsyncAudioResampler
    where
        Self: Sized + Send + 'static,
    {
        SyncToAsyncResamplerAdapter::new(self)
    }
}

pub trait AsyncAudioResampler {
    fn request_resample(
        &self,
        frames: AudioBuffer,
        sample_rate: SampleRate,
        pitch: f64,
        callback: impl FnOnce(Option<AudioBuffer>) + Send + 'static,
    );

    fn into_sync(self) -> impl SyncAudioResampler
    where
        Self: Sized + Send + 'static,
    {
        AsyncToSyncResamplerAdapter::new(self)
    }
}

struct SyncToAsyncResamplerPayload(
    AudioBuffer,
    SampleRate,
    f64,
    Box<dyn FnOnce(Option<AudioBuffer>) + Send + 'static>,
);

pub struct SyncToAsyncResamplerAdapter<T: SyncAudioResampler + Send + 'static> {
    sender: Option<Sender<SyncToAsyncResamplerPayload>>,
    join_handle: Option<thread::JoinHandle<T>>,
}

impl<T: SyncAudioResampler + Send + 'static> SyncToAsyncResamplerAdapter<T> {
    pub fn new(resampler: T) -> Self {
        let (sender, receiver) = unbounded::<SyncToAsyncResamplerPayload>();
        let sender = Some(sender);
        let join_handle = thread::spawn(move || {
            loop {
                let SyncToAsyncResamplerPayload(frames, sample_rate, pitch, callback) =
                    match receiver.recv() {
                        Ok(data) => data,
                        Err(_) => break,
                    };
                let result = resampler.resample(frames, sample_rate, pitch);
                callback(result);
            }
            return resampler;
        });
        let join_handle = Some(join_handle);
        SyncToAsyncResamplerAdapter {
            sender,
            join_handle,
        }
    }
}

impl<T: SyncAudioResampler + Send> AsyncAudioResampler for SyncToAsyncResamplerAdapter<T> {
    fn request_resample(
        &self,
        frames: AudioBuffer,
        sample_rate: SampleRate,
        pitch: f64,
        callback: impl FnOnce(Option<AudioBuffer>) + Send + 'static,
    ) {
        //* senderがNoneになるのはinto_syncが呼ばれた際のみで、その時にはselfを消費するので、もうrequest_resampleを呼ぶことは出来ず、ここに到達することはないため、unwrapしても安全。
        let result = self
            .sender
            .as_ref()
            .unwrap()
            .send(SyncToAsyncResamplerPayload(
                frames,
                sample_rate,
                pitch,
                Box::new(callback),
            ));
        if let Err(SendError(SyncToAsyncResamplerPayload(_, _, _, callback))) = result {
            callback(None);
        }
    }

    fn into_sync(mut self) -> impl SyncAudioResampler
    where
        Self: Sized + Send + 'static,
    {
        self.sender.take();
        //* join_handleがNoneになるのはdropが呼ばれた際か今から呼ぶ部分のみで、その時にはselfを消費するので、もうinto_syncを呼ぶことは出来ず、ここに到達することはないため、unwrapしても安全。
        //* Dropの方では結果を返す必要が無いのでif letでunwrapせずにjoinしているが、ここでは結果を返す必要があるのでunwrapしている。
        self.join_handle
            .take()
            .unwrap()
            .join()
            .expect("Resampler thread panicked! This is a bug in the resampler implementation.")
    }
}

impl<T: SyncAudioResampler + Send + 'static> Drop for SyncToAsyncResamplerAdapter<T> {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.join().expect(
                "Resampler thread panicked! This is a bug in the resampler implementation.",
            );
        }
    }
}

pub struct AsyncToSyncResamplerAdapter<T: AsyncAudioResampler> {
    resampler: T,
}

impl<T: AsyncAudioResampler> AsyncToSyncResamplerAdapter<T> {
    pub fn new(resampler: T) -> Self {
        AsyncToSyncResamplerAdapter { resampler }
    }
}

impl<T: AsyncAudioResampler> SyncAudioResampler for AsyncToSyncResamplerAdapter<T> {
    fn resample(&self, frames: AudioBuffer, sample_rate: SampleRate, pitch: f64) -> Option<AudioBuffer> {
        let (sender, receiver) = unbounded();
        self.resampler
            .request_resample(frames, sample_rate, pitch, move |result| {
                let _ = sender.send(result);
            });
        receiver.recv().ok().flatten()
    }

    fn into_async(self) -> impl AsyncAudioResampler
    where
        Self: Sized + Send + 'static,
    {
        self.resampler
    }
}

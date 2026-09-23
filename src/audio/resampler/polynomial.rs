use rubato::{
    Async, FixedAsync, PolynomialDegree, Resampler, audioadapter_buffers::direct::InterleavedSlice,
};

use crate::audio::{AudioBuffer, SampleRate, resampler::SyncAudioResampler};

#[derive(Debug, Clone, Copy)]
pub enum InterpolationType {
    Nearest,
    Linear,
    Cubic,
    Quintic,
    Septic,
}

impl Into<PolynomialDegree> for InterpolationType {
    fn into(self) -> PolynomialDegree {
        match self {
            InterpolationType::Nearest => PolynomialDegree::Nearest,
            InterpolationType::Linear => PolynomialDegree::Linear,
            InterpolationType::Cubic => PolynomialDegree::Cubic,
            InterpolationType::Quintic => PolynomialDegree::Quintic,
            InterpolationType::Septic => PolynomialDegree::Septic,
        }
    }
}

pub struct PolynomialResampler(InterpolationType);

impl PolynomialResampler {
    pub fn new(interpolation_type: InterpolationType) -> Self {
        PolynomialResampler(interpolation_type)
    }
}

impl SyncAudioResampler for PolynomialResampler {
    fn resample(
        &self,
        frames: AudioBuffer,
        sample_rate: SampleRate,
        pitch: f64,
    ) -> Option<AudioBuffer> {
        let frame_count = frames.len();
        if frame_count == 0 {
            return Some(AudioBuffer::from_vec(Vec::new(), sample_rate));
        }
        let resample_ratio = sample_rate.get() as f64 / (frames.sample_rate().get() as f64 * pitch);
        let mut resampler = Async::<f32>::new_poly(
            resample_ratio,
            1.0,
            self.0.into(),
            1024,
            2,
            FixedAsync::Input,
        )
        .ok()?;
        let buf_in = InterleavedSlice::new(frames.as_flattened(), 2, frame_count).ok()?;
        let buf_out = resampler.process_all(&buf_in, frame_count, None).ok()?;
        let buf_out = {
            let mut buf_out = buf_out.take_data();
            let (len, cap) = (buf_out.len(), buf_out.capacity());
            let len_rem = len % 2;
            if len_rem != 0 {
                buf_out.truncate(len - len_rem);
            }
            let cap_rem = cap % 2;
            if cap_rem != 0 {
                buf_out.shrink_to(cap - cap_rem);
            }
            let (ptr, _, _) = buf_out.into_raw_parts();
            //* lenとcapは2で割り切れるように調整済みなので、f32のVecを安全に[f32; 2]のVecに変換出来る。
            unsafe { Vec::from_raw_parts(ptr.cast(), len / 2, cap / 2) }
        };
        let frames = AudioBuffer::from_vec(buf_out, sample_rate);
        Some(frames)
    }
}

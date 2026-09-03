use rubato::{
    Async, FixedAsync, PolynomialDegree, Resampler, audioadapter_buffers::direct::InterleavedSlice,
};

use crate::audio::{Frame, Frames, SampleRate, resampler::SyncAudioResampler};

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
    fn resample(&self, frames: Frames, sample_rate: SampleRate, pitch: f64) -> Option<Frames> {
        let frame_count = frames.len();
        if frame_count == 0 {
            return Some(Frames::from_vec(Vec::new(), sample_rate));
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
        //* fftにチャンネル数を2として設定しているため、buf_outのlen、capともに1/2になる。ptrはFrameにキャストし、Vecとして再構築する。
        let buf_out = {
            let (ptr, len, cap) = buf_out.take_data().into_raw_parts();
            let ptr = ptr as *mut Frame;
            let len = len / 2;
            let cap = cap / 2;
            unsafe { Vec::from_raw_parts(ptr, len, cap) }
        };
        let frames = Frames::from_vec(buf_out, sample_rate);
        Some(frames)
    }
}

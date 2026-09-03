use rubato::{
    Async, FixedAsync, PolynomialDegree, Resampler, audioadapter_buffers::direct::InterleavedSlice,
};

use crate::audio::{Frame, SampleRate, instrument::InstrumentAudio};

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

pub fn resample_audio(
    audio: &InstrumentAudio,
    pitch: f64,
    sample_rate: SampleRate,
    interpolation_type: InterpolationType,
) -> Option<Vec<Frame>> {
    let frame_count = audio.frame_count();
    if frame_count == 0 {
        return Some(Vec::new());
    }
    let resample_ratio = sample_rate.get() as f64 / (audio.sample_rate().get() as f64 * pitch);
    let mut resampler = Async::<f32>::new_poly(
        resample_ratio,
        1.0,
        interpolation_type.into(),
        1024,
        2,
        FixedAsync::Input,
    )
    .ok()?;
    let buf_in = InterleavedSlice::new(audio.frames().as_flattened(), 2, frame_count).ok()?;
    let buf_out = resampler.process_all(&buf_in, frame_count, None).ok()?;
    //* fftにチャンネル数を2として設定しているため、buf_outのlen、capともに1/2になる。ptrはFrameにキャストし、Vecとして再構築する。
    let buf_out = {
        let (ptr, len, cap) = buf_out.take_data().into_raw_parts();
        let ptr = ptr as *mut Frame;
        let len = len / 2;
        let cap = cap / 2;
        unsafe { Vec::from_raw_parts(ptr, len, cap) }
    };
    Some(buf_out)
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU16, NonZeroU32};

    use crate::audio::InstrumentAudio;

    use super::*;

    #[test]
    fn resampling_does_not_keep_large_fft_delay_at_the_start() {
        let source_sample_rate = NonZeroU32::new(44_100).unwrap();
        let target_sample_rate = NonZeroU32::new(48_000).unwrap();
        let channels = NonZeroU16::new(2).unwrap();

        let mut samples = vec![0.0; 12_000 * 2];
        samples[0] = 1.0;
        samples[1] = 1.0;

        let audio = InstrumentAudio::new(samples, channels, source_sample_rate);
        let pitch = 2.0f64.powf(1.0 / 12.0);
        let frames =
            resample_audio(&audio, pitch, target_sample_rate, InterpolationType::Cubic).unwrap();

        let first_audible = frames
            .iter()
            .position(|[l, r]| l.abs().max(r.abs()) > 0.0001)
            .unwrap();

        assert!(
            first_audible < 512,
            "first audible frame was delayed to {first_audible}"
        );
    }
}

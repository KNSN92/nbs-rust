pub mod polynomial;

use crate::audio::{Frames, SampleRate, instrument::InstrumentAudio};

pub trait NoteAudioResampler {
    fn resample(
        &self,
        audio: InstrumentAudio,
        sample_rate: SampleRate,
        pitch: f64,
    ) -> Option<Frames>;
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

        let audio = InstrumentAudio::new(&samples, channels, source_sample_rate);
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

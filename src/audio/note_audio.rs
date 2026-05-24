use std::{num::NonZeroU32, sync::Arc};

use rubato::{
    Async, FixedAsync, Indexing, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction, audioadapter_buffers::direct::InterleavedSlice, calculate_cutoff,
};

use crate::{
    audio::{Float, Frame, InstrumentAudio, Sample, SampleRate, provider::InstrumentAudioProvider},
    instrument::CustomInstrument,
    noteblock::{Layer, Note},
};

pub struct NoteAudio {
    frames: Arc<[Frame]>,
    volume: Float,
    panning: [Float; 2],
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
        let volume = volume(note, layer);
        let panning = panning(note, layer);

        let frames = resample_audio(&audio, pitch, sample_rate)?;
        Some(NoteAudio {
            frames: frames.into(),
            volume,
            panning,
            sample_rate,
            pos: 0,
        })
    }

    #[inline]
    pub fn sample_rate(&self) -> NonZeroU32 {
        self.sample_rate
    }

    pub fn for_note(&self, note: &Note, layer: Option<&Layer>) -> Self {
        NoteAudio {
            frames: self.frames.clone(),
            volume: volume(note, layer),
            panning: panning(note, layer),
            sample_rate: self.sample_rate,
            pos: 0,
        }
    }

    #[inline]
    fn apply_effect(&self, [l, r]: Frame) -> Frame {
        [
            l * self.panning[0] * self.volume,
            r * self.panning[1] * self.volume,
        ]
    }
}

fn volume(note: &Note, layer: Option<&Layer>) -> Float {
    let layer_volume = layer
        .map(|layer| layer.volume as Float / 100.0)
        .unwrap_or(1.0);
    let note_volume = note.volume as Float / 100.0;
    note_volume * layer_volume
}

fn panning(note: &Note, layer: Option<&Layer>) -> [Float; 2] {
    let layer_panning = layer.map(|l| l.panning as Float / 100.0).unwrap_or(0.0);
    let note_panning = note.panning as Float / 100.0;
    let panning = match layer_panning {
        0.0 => note_panning,
        _ => (layer_panning + note_panning) / 2.0,
    };
    [2.0 - panning, panning]
}

fn pitch(note: &Note, custom_instrument: Option<&CustomInstrument>) -> f64 {
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
        if let Some(frame) = self.frames.get(self.pos) {
            self.pos += 1;
            Some(self.apply_effect(*frame))
        } else {
            None
        }
    }
}

fn resample_audio(
    audio: &InstrumentAudio,
    pitch: f64,
    sample_rate: SampleRate,
) -> Option<Vec<Frame>> {
    let input_len = audio.frame_count();
    if input_len == 0 {
        return Some(Vec::new());
    }

    let resample_ratio = sample_rate.get() as f64 / (audio.sample_rate().get() as f64 * pitch);
    if !resample_ratio.is_finite() || resample_ratio <= 0.0 {
        return None;
    }

    let sinc_len = 64;
    let window = WindowFunction::BlackmanHarris2;
    let params = SincInterpolationParameters {
        sinc_len,
        f_cutoff: calculate_cutoff::<f32>(sinc_len, window),
        interpolation: SincInterpolationType::Cubic,
        oversampling_factor: 128,
        window,
    };

    let mut resampler =
        Async::<Sample>::new_sinc(resample_ratio, 1.0, &params, 1024, 2, FixedAsync::Input).ok()?;

    let expected_output_len = (input_len as f64 * resample_ratio).ceil() as usize;
    let delay = resampler.output_delay();
    let output_capacity = resampler.process_all_needed_output_len(input_len);
    let mut frames = vec![[0.0; 2]; output_capacity];

    let input_audio = InterleavedSlice::new(audio.frames().as_flattened(), 2, input_len).ok()?;
    let mut output_audio =
        InterleavedSlice::new_mut(frames.as_flattened_mut(), 2, output_capacity).ok()?;

    let mut input_offset = 0;
    let mut output_offset = 0;
    let mut input_frames_left = input_len;

    while input_frames_left > 0 {
        let input_frames_next = resampler.input_frames_next();
        let partial_len = (input_frames_left < input_frames_next).then_some(input_frames_left);
        let indexing = Indexing {
            input_offset,
            output_offset,
            partial_len,
            active_channels_mask: None,
        };
        let (input_frames, output_frames) = resampler
            .process_into_buffer(&input_audio, &mut output_audio, Some(&indexing))
            .ok()?;
        let consumed = partial_len.unwrap_or(input_frames).min(input_frames_left);
        input_offset += consumed;
        input_frames_left -= consumed;
        output_offset += output_frames;
    }

    while output_offset < delay + expected_output_len {
        let indexing = Indexing {
            input_offset,
            output_offset,
            partial_len: Some(0),
            active_channels_mask: None,
        };
        let (_, output_frames) = resampler
            .process_into_buffer(&input_audio, &mut output_audio, Some(&indexing))
            .ok()?;
        if output_frames == 0 {
            return None;
        }
        output_offset += output_frames;
    }

    drop(output_audio);

    frames.copy_within(delay..delay + expected_output_len, 0);
    frames.truncate(expected_output_len);
    Some(frames)
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
        let frames = resample_audio(&audio, pitch, target_sample_rate).unwrap();

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

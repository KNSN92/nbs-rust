use std::io;

use symphonia::{
    core::{
        audio::{AudioBufferRef, SampleBuffer, SignalSpec},
        codecs::{CODEC_TYPE_NULL, CodecParameters, Decoder, DecoderOptions},
        errors::Error,
        formats::{FormatOptions, FormatReader},
        io::{MediaSource, MediaSourceStream},
        meta::MetadataOptions,
        probe::Hint,
        units,
    },
    default::{get_codecs, get_probe},
};
use thiserror::Error;

use crate::audio::{Channels, SampleRate, instrument::InstrumentAudio};

#[derive(Debug, Error)]
pub enum DecodeAudioError {
    #[error("Unsupported audio format")]
    UnsupportedFormat,
    #[error("Unsupported audio codec")]
    UnsupportedCodec,
    #[error("No audio streams found")]
    NoStreams,
    #[error("No audio samples decoded")]
    NoAudioData,
    #[error("IO error: {0}")]
    IOError(#[from] io::Error),
    #[error("Uncategorized error: {0}")]
    Uncategorized(String),
    #[error("Number of channels is out of range")]
    ChannelsOutOfRange,
    #[error("Sample rate is zero")]
    ZeroSampleRate,
    #[error("Audio stream changed channels or sample rate during decoding")]
    InconsistentAudioSpec,
}

pub fn decode_audio(
    data: impl MediaSource + 'static,
    hint_ext: Option<&str>,
) -> Result<InstrumentAudio, DecodeAudioError> {
    let mut format = probe_format(data, hint_ext)?;
    let track = select_track(&*format)?;
    let decoder_opts = DecoderOptions::default();
    let mut decoder = get_codecs()
        .make(&track.codec_params, &decoder_opts)
        .map_err(|_| DecodeAudioError::UnsupportedCodec)?;
    let decoded = decode_track(&mut *format, &mut *decoder, track.id)?;

    Ok(InstrumentAudio::new(
        decoded.samples,
        decoded.spec.channels,
        decoded.spec.sample_rate,
    ))
}

fn probe_format(
    data: impl MediaSource + 'static,
    hint_ext: Option<&str>,
) -> Result<Box<dyn FormatReader>, DecodeAudioError> {
    let mss = MediaSourceStream::new(Box::new(data), Default::default());
    let mut hint = Hint::new();
    if let Some(hint_ext) = hint_ext {
        hint.with_extension(hint_ext);
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let format = get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|_| DecodeAudioError::UnsupportedFormat)?
        .format;

    if format.tracks().is_empty() {
        return Err(DecodeAudioError::NoStreams);
    }

    Ok(format)
}

struct SelectedTrack {
    id: u32,
    codec_params: CodecParameters,
}

fn select_track(format: &dyn FormatReader) -> Result<SelectedTrack, DecodeAudioError> {
    let track = format
        .default_track()
        .filter(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        .or_else(|| {
            format
                .tracks()
                .iter()
                .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        })
        .ok_or(DecodeAudioError::UnsupportedCodec)?;

    Ok(SelectedTrack {
        id: track.id,
        codec_params: track.codec_params.clone(),
    })
}

struct DecodedAudio {
    samples: Vec<f32>,
    spec: DecodedAudioSpec,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct DecodedAudioSpec {
    channels: Channels,
    sample_rate: SampleRate,
}

impl DecodedAudioSpec {
    fn try_from_symphonia(spec: &SignalSpec) -> Result<Self, DecodeAudioError> {
        let channels = Channels::new(
            spec.channels
                .count()
                .try_into()
                .map_err(|_| DecodeAudioError::ChannelsOutOfRange)?,
        )
        .ok_or(DecodeAudioError::ChannelsOutOfRange)?;

        let sample_rate = SampleRate::new(spec.rate).ok_or(DecodeAudioError::ZeroSampleRate)?;

        Ok(Self {
            channels,
            sample_rate,
        })
    }
}

fn decode_track(
    format: &mut dyn FormatReader,
    decoder: &mut dyn Decoder,
    track_id: u32,
) -> Result<DecodedAudio, DecodeAudioError> {
    let mut samples = Vec::new();
    let mut audio_spec = None;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(err)) if err.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(err) => return Err(map_symphonia_error(err)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(Error::DecodeError(_) | Error::IoError(_)) => continue,
            Err(err) => return Err(map_symphonia_error(err)),
        };

        append_decoded_audio(decoded, &mut samples, &mut audio_spec)?;
    }

    let spec = audio_spec.ok_or(DecodeAudioError::NoAudioData)?;

    Ok(DecodedAudio { samples, spec })
}

fn append_decoded_audio(
    decoded: AudioBufferRef<'_>,
    samples: &mut Vec<f32>,
    audio_spec: &mut Option<DecodedAudioSpec>,
) -> Result<(), DecodeAudioError> {
    if decoded.frames() == 0 {
        return Ok(());
    }

    let decoded_spec = DecodedAudioSpec::try_from_symphonia(decoded.spec())?;
    match audio_spec {
        Some(spec) if *spec != decoded_spec => {
            return Err(DecodeAudioError::InconsistentAudioSpec);
        }
        Some(_) => {}
        None => *audio_spec = Some(decoded_spec),
    }

    let duration = units::Duration::from(decoded.capacity() as u64);
    let mut sample_buffer = SampleBuffer::new(duration, *decoded.spec());
    sample_buffer.copy_interleaved_ref(decoded);
    samples.extend_from_slice(sample_buffer.samples());

    Ok(())
}

fn map_symphonia_error(err: Error) -> DecodeAudioError {
    match err {
        Error::IoError(err) => DecodeAudioError::IOError(err),
        Error::Unsupported(_) => DecodeAudioError::UnsupportedFormat,
        err => DecodeAudioError::Uncategorized(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::audio::InstrumentAudio;

    #[test]
    fn decodes_vanilla_ogg_audio() {
        let audio =
            InstrumentAudio::from_bytes(include_bytes!("../../audio/harp.ogg"), Some("ogg"))
                .expect("vanilla OGG should decode");

        assert!(audio.frame_count() > 0);
        assert!(audio.sample_rate().get() > 0);
    }
}

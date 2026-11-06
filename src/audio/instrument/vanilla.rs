use std::sync::LazyLock;

use crate::{audio::AudioBuffer, instrument::VANILLA_INSTRUMENT_COUNT};

fn decode_vorbis(vorbis_bin: &'static [u8]) -> AudioBuffer {
    AudioBuffer::from_file_bytes(vorbis_bin, Some("ogg")).unwrap()
}

macro_rules! audio {
    ($file:expr $(,)?) => {
        include_bytes!(concat!("../../../audio/", $file))
    };
}

pub static VANILLA_AUDIOS: LazyLock<[AudioBuffer; VANILLA_INSTRUMENT_COUNT as usize]> =
    LazyLock::new(|| {
        [
            decode_vorbis(audio!("harp.ogg")),
            decode_vorbis(audio!("dbass.ogg")),
            decode_vorbis(audio!("bdrum.ogg")),
            decode_vorbis(audio!("sdrum.ogg")),
            decode_vorbis(audio!("click.ogg")),
            decode_vorbis(audio!("guitar.ogg")),
            decode_vorbis(audio!("flute.ogg")),
            decode_vorbis(audio!("bell.ogg")),
            decode_vorbis(audio!("icechime.ogg")),
            decode_vorbis(audio!("xylobone.ogg")),
            decode_vorbis(audio!("iron_xylophone.ogg")),
            decode_vorbis(audio!("cow_bell.ogg")),
            decode_vorbis(audio!("didgeridoo.ogg")),
            decode_vorbis(audio!("bit.ogg")),
            decode_vorbis(audio!("banjo.ogg")),
            decode_vorbis(audio!("pling.ogg")),
            decode_vorbis(audio!("trumpet.ogg")),
            decode_vorbis(audio!("trumpet_exposed.ogg")),
            decode_vorbis(audio!("trumpet_weathered.ogg")),
            decode_vorbis(audio!("trumpet_oxidized.ogg")),
        ]
    });

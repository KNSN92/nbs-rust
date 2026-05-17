use std::sync::LazyLock;

use crate::{audio::InstrumentAudio, instrument::VANILLA_INSTRUMENT_COUNT};

fn decode_vorbis(vorbis_bin: &'static [u8]) -> InstrumentAudio {
    InstrumentAudio::from_bytes(vorbis_bin, Some("ogg")).unwrap()
}

macro_rules! audio {
    ($file:expr $(,)?) => {
        include_bytes!(concat!("../../audio/", $file))
    };
}

pub static VANILLA_AUDIOS: LazyLock<[InstrumentAudio; VANILLA_INSTRUMENT_COUNT as usize]> =
    LazyLock::new(|| {
        [
            decode_vorbis(audio!("harp.ogg")),
            decode_vorbis(audio!("double_bass.ogg")),
            decode_vorbis(audio!("bass_drum.ogg")),
            decode_vorbis(audio!("snare_drum.ogg")),
            decode_vorbis(audio!("click.ogg")),
            decode_vorbis(audio!("guitar.ogg")),
            decode_vorbis(audio!("flute.ogg")),
            decode_vorbis(audio!("bell.ogg")),
            decode_vorbis(audio!("chime.ogg")),
            decode_vorbis(audio!("xylophone.ogg")),
            decode_vorbis(audio!("iron_xylophone.ogg")),
            decode_vorbis(audio!("cow_bell.ogg")),
            decode_vorbis(audio!("didgeridoo.ogg")),
            decode_vorbis(audio!("bit.ogg")),
            decode_vorbis(audio!("banjo.ogg")),
            decode_vorbis(audio!("pling.ogg")),
        ]
    });

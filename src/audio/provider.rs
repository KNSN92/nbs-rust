use std::{fs::File, path::Path};

use crate::{
    Instrument,
    audio::{InstrumentAudio, decoder::decode_audio_from_file, vanilla_audio::VANILLA_AUDIOS},
    instrument::{InstrumentSet, VANILLA_INSTRUMENT_COUNT},
};

pub trait InstrumentAudioProvider {
    fn get_audio(&self, instrument: Instrument) -> Option<InstrumentAudio>;
}

pub struct VanillaAudioProvider {
    vanilla_instruments: u8,
}

impl VanillaAudioProvider {
    pub fn new(vanilla_instruments: u8) -> Self {
        VanillaAudioProvider {
            vanilla_instruments,
        }
    }
}

impl Default for VanillaAudioProvider {
    fn default() -> Self {
        VanillaAudioProvider {
            vanilla_instruments: VANILLA_INSTRUMENT_COUNT,
        }
    }
}

impl InstrumentAudioProvider for VanillaAudioProvider {
    fn get_audio(&self, instrument: Instrument) -> Option<InstrumentAudio> {
        let id = instrument.0;
        if id >= self.vanilla_instruments {
            return None;
        }
        VANILLA_AUDIOS.get(id as usize).cloned()
    }
}

pub struct FileAudioProvider {
    vanilla_instruments: u8,
    custom_instrument_audios: Vec<Option<InstrumentAudio>>,
}

impl FileAudioProvider {
    pub fn from_directory(
        dir: impl AsRef<Path>,
        instrument_set: &InstrumentSet,
        vanilla_instruments: u8,
    ) -> Self {
        let mut custom_instrument_audios =
            Vec::with_capacity(instrument_set.custom_instrument_count() as usize);
        for ci in instrument_set.as_slice() {
            let path = dir.as_ref().join(&ci.file_name);
            let audio = File::open(path)
                .ok()
                .and_then(|file| decode_audio_from_file(file, None).ok());
            #[cfg(debug_assertions)]
            if audio.is_none() {
                eprintln!(
                    "Warning: Failed to load audio for custom instrument '{}' from file '{}'",
                    ci.name, ci.file_name
                );
            }
            custom_instrument_audios.push(audio);
        }
        FileAudioProvider {
            vanilla_instruments,
            custom_instrument_audios,
        }
    }

    #[allow(dead_code, unused)]
    pub fn from_zip(
        path: impl AsRef<Path>,
        custom_instruments: &InstrumentSet,
        vanilla_instruments: u8,
    ) -> Self {
        todo!()
    }
}

impl InstrumentAudioProvider for FileAudioProvider {
    fn get_audio(&self, instrument: Instrument) -> Option<InstrumentAudio> {
        let id = instrument.0;
        if id < self.vanilla_instruments {
            VANILLA_AUDIOS.get(id as usize).cloned()
        } else {
            self.custom_instrument_audios
                .get((id - self.vanilla_instruments) as usize)
                .cloned()
                .flatten()
        }
    }
}

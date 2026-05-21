use std::{fs::File, path::Path};

use crate::{
    Instrument,
    audio::{InstrumentAudio, vanilla_audio::VANILLA_AUDIOS},
    instrument::{CustomInstrument, InstrumentSet, TEMPO_CHANGER, VANILLA_INSTRUMENT_COUNT},
};

pub trait InstrumentAudioProvider {
    fn get_audio(&self, instrument: Instrument) -> Option<InstrumentAudio>;
}

impl InstrumentAudioProvider for Box<dyn InstrumentAudioProvider + Send> {
    fn get_audio(&self, instrument: Instrument) -> Option<InstrumentAudio> {
        self.as_ref().get_audio(instrument)
    }
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
    ) -> (Self, Vec<&CustomInstrument>) {
        let mut custom_instrument_audios =
            Vec::with_capacity(instrument_set.custom_instrument_count() as usize);
        let mut failed_custom_instruments =
            Vec::with_capacity(instrument_set.custom_instrument_count() as usize);
        for ci in instrument_set.as_slice() {
            if ci.name == TEMPO_CHANGER {
                custom_instrument_audios.push(None);
                continue;
            }
            let path = dir.as_ref().join(&ci.file_name);
            let audio = File::open(path)
                .ok()
                .and_then(|file| InstrumentAudio::from_file(file, None).ok());
            if audio.is_none() {
                failed_custom_instruments.push(ci);
            }
            custom_instrument_audios.push(audio);
        }
        (
            FileAudioProvider {
                vanilla_instruments,
                custom_instrument_audios,
            },
            failed_custom_instruments,
        )
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

use crate::{Instrument, instrument::VANILLA_INSTRUMENT_COUNT};

#[derive(Debug)]
pub struct CustomInstrument {
    pub name: String,
    pub file_name: String,
    pub key: u8,
    pub press_piano_key: bool,
}

impl Default for CustomInstrument {
    fn default() -> Self {
        CustomInstrument {
            name: "".to_string(),
            file_name: "".to_string(),
            key: 45,
            press_piano_key: true,
        }
    }
}

#[derive(Debug)]
pub struct InstrumentSet {
    custom_instruments: Vec<CustomInstrument>,
    vanilla_instruments: u8,
}

impl InstrumentSet {
    pub fn new(vanilla_instruments: u8) -> Self {
        InstrumentSet {
            custom_instruments: vec![],
            vanilla_instruments,
        }
    }

    pub fn is_tempo_changer(&self, instrument: Instrument) -> bool {
        self.get(instrument)
            .map(|ci| &ci.name == "Tempo Changer")
            .unwrap_or(false)
    }

    pub fn set_vanilla_instrument_count(&mut self, count: u8) {
        self.vanilla_instruments = count;
    }

    pub fn get(&self, instrument: Instrument) -> Option<&CustomInstrument> {
        match instrument {
            Instrument(id) if id >= self.vanilla_instruments => self
                .custom_instruments
                .get((id - self.vanilla_instruments) as usize),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, instrument: Instrument) -> Option<&mut CustomInstrument> {
        match instrument {
            Instrument(id) if id >= self.vanilla_instruments => self
                .custom_instruments
                .get_mut((id - self.vanilla_instruments) as usize),
            _ => None,
        }
    }

    pub fn push(&mut self, custom_instrument: CustomInstrument) -> Result<(), CustomInstrument> {
        if self.custom_instruments.len() + self.vanilla_instruments as usize >= 256 {
            return Err(custom_instrument);
        }
        self.custom_instruments.push(custom_instrument);
        Ok(())
    }

    pub fn as_slice(&self) -> &[CustomInstrument] {
        &self.custom_instruments
    }

    pub fn as_slice_mut(&mut self) -> &mut [CustomInstrument] {
        &mut self.custom_instruments
    }

    pub fn vanilla_instrument_count(&self) -> u8 {
        self.vanilla_instruments
    }

    pub fn custom_instrument_count(&self) -> u8 {
        self.custom_instruments.len() as u8
    }
}

impl Default for InstrumentSet {
    fn default() -> Self {
        InstrumentSet {
            custom_instruments: vec![],
            vanilla_instruments: VANILLA_INSTRUMENT_COUNT,
        }
    }
}

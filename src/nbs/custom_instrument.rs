use crate::Instrument;

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
pub struct CustomInstruments {
    instruments: Vec<CustomInstrument>,
    vanilla_instruments: u8,
}

impl CustomInstruments {
    pub fn new(vanilla_instruments: u8) -> Self {
        CustomInstruments {
            instruments: vec![],
            vanilla_instruments,
        }
    }

    pub fn is_tempo_changer(&self, instrument: Instrument) -> bool {
        self.get(instrument)
            .map(|ci| &ci.name == "Tempo Changer")
            .unwrap_or(false)
    }

    pub fn get(&self, instrument: Instrument) -> Option<&CustomInstrument> {
        match instrument {
            Instrument(id) if id >= self.vanilla_instruments => self
                .instruments
                .get((id - self.vanilla_instruments) as usize),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, instrument: Instrument) -> Option<&mut CustomInstrument> {
        match instrument {
            Instrument(id) if id >= self.vanilla_instruments => self
                .instruments
                .get_mut((id - self.vanilla_instruments) as usize),
            _ => None,
        }
    }

    pub fn push(&mut self, custom_instrument: CustomInstrument) -> Result<(), CustomInstrument> {
        if self.instruments.len() + self.vanilla_instruments as usize >= 256 {
            return Err(custom_instrument);
        }
        self.instruments.push(custom_instrument);
        Ok(())
    }

    pub fn instruments(&self) -> &[CustomInstrument] {
        &self.instruments
    }

    pub fn instruments_mut(&mut self) -> &mut [CustomInstrument] {
        &mut self.instruments
    }

    pub(crate) fn into_vec(self) -> Vec<CustomInstrument> {
        self.instruments
    }

    pub fn vanilla_instrument_count(&self) -> u8 {
        self.vanilla_instruments
    }

    pub fn count(&self) -> u8 {
        self.instruments.len() as u8
    }
}

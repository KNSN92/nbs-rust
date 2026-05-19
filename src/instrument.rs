#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instrument(pub u8);

pub const VANILLA_INSTRUMENT_COUNT: u8 = 16;

#[allow(non_upper_case_globals)]
impl Instrument {
    /// The harp instrument, id 0.
    pub const Harp: Instrument = Instrument(0);
    /// The double bass instrument, id 1.
    pub const DoubleBass: Instrument = Instrument(1);
    /// The bass drum instrument, id 2.
    pub const BassDrum: Instrument = Instrument(2);
    /// The snare drum instrument, id 3.
    pub const SnareDrum: Instrument = Instrument(3);
    /// The click instrument, id 4.
    pub const Click: Instrument = Instrument(4);
    /// The guitar instrument, id 5.
    pub const Guitar: Instrument = Instrument(5);
    /// The flute instrument, id 6.
    pub const Flute: Instrument = Instrument(6);
    /// The bell instrument, id 7.
    pub const Bell: Instrument = Instrument(7);
    /// The chime instrument, id 8.
    pub const Chime: Instrument = Instrument(8);
    /// The xylophone instrument, id 9.
    pub const Xylophone: Instrument = Instrument(9);
    /// The iron xylophone instrument, id 10.
    pub const IronXylophone: Instrument = Instrument(10);
    /// The cow bell instrument, id 11.
    pub const CowBell: Instrument = Instrument(11);
    /// The didgeridoo instrument, id 12.
    pub const Didgeridoo: Instrument = Instrument(12);
    /// The bit instrument, id 13.
    pub const Bit: Instrument = Instrument(13);
    /// The banjo instrument, id 14.
    pub const Banjo: Instrument = Instrument(14);
    /// The pling instrument, id 15.
    pub const Pling: Instrument = Instrument(15);
}

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

pub const TEMPO_CHANGER: &str = "Tempo Changer";

impl InstrumentSet {
    pub fn new(vanilla_instruments: u8) -> Self {
        InstrumentSet {
            custom_instruments: vec![],
            vanilla_instruments,
        }
    }

    pub fn is_tempo_changer(&self, instrument: Instrument) -> bool {
        self.get(instrument)
            .map(|ci| &ci.name == TEMPO_CHANGER)
            .unwrap_or(false)
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

    pub fn push(
        &mut self,
        custom_instrument: CustomInstrument,
    ) -> Result<Instrument, CustomInstrument> {
        if self.custom_instruments.len() + self.vanilla_instruments as usize >= 256 {
            return Err(custom_instrument);
        }
        let instrument = Instrument(self.instrument_count());
        self.custom_instruments.push(custom_instrument);
        Ok(instrument)
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

    pub fn instrument_count(&self) -> u8 {
        self.vanilla_instrument_count() + self.custom_instrument_count()
    }

    pub fn has_custom_instrument(&self) -> bool {
        !self.custom_instruments.is_empty()
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

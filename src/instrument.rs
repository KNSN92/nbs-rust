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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instrument(pub u8);

pub const VANILLA_INSTRUMENT_COUNT: u8 = 16;

#[allow(non_upper_case_globals)]
impl Instrument {
    pub const Harp: Instrument = Instrument(0);
    pub const DoubleBass: Instrument = Instrument(1);
    pub const BassDrum: Instrument = Instrument(2);
    pub const SnareDrum: Instrument = Instrument(3);
    pub const Click: Instrument = Instrument(4);
    pub const Guitar: Instrument = Instrument(5);
    pub const Flute: Instrument = Instrument(6);
    pub const Bell: Instrument = Instrument(7);
    pub const Chime: Instrument = Instrument(8);
    pub const Xylophone: Instrument = Instrument(9);
    pub const IronXylophone: Instrument = Instrument(10);
    pub const CowBell: Instrument = Instrument(11);
    pub const Didgeridoo: Instrument = Instrument(12);
    pub const Bit: Instrument = Instrument(13);
    pub const Banjo: Instrument = Instrument(14);
    pub const Pling: Instrument = Instrument(15);
}

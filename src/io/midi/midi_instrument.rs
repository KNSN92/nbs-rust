// https://gist.github.com/u3002/cf4daa83bc82b5917fc86fb23815578a

use crate::instrument::Instrument;

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub struct MidiInstrument {
    pub name: &'static str,
    pub instrument: Instrument,
    pub octave: i8,
    pub short_name: Option<&'static str>,
}

pub struct MidiInstrumentSet {
    pub override_instruments: [Option<(Instrument, i8)>; MIDI_INSTRUMENTS.len()],
    pub override_drums: [Option<(Instrument, i8)>; MIDI_DRUMS.len()],
}

impl MidiInstrumentSet {
    pub fn new() -> Self {
        MidiInstrumentSet {
            override_instruments: [None; MIDI_INSTRUMENTS.len()],
            override_drums: [None; MIDI_DRUMS.len()]
        }
    }

    pub fn override_instrument(&mut self, id: usize, instrument: Instrument, octave: i8) {
        if id < self.override_instruments.len() {
            self.override_instruments[id] = Some((instrument, octave));
        }
    }

    pub fn override_drum(&mut self, id: usize, instrument: Instrument, key: i8) {
        if id < self.override_drums.len() {
            self.override_drums[id] = Some((instrument, key));
        }
    }

    pub(crate) fn get_instrument(&self, id: usize) -> Option<MidiInstrument> {
        let mut ins = *MIDI_INSTRUMENTS.get(id)?;
        if let Some(ins_override) = self.override_instruments.get(id).copied().flatten() {
            ins.instrument = ins_override.0;
            ins.octave = ins_override.1;
        }
        Some(ins)
    }

    pub(crate) fn get_drum(&self, id: usize) -> Option<MidiPercussion> {
        let mut drum = *MIDI_DRUMS.get(id - 24)?; // MIDI percussion key starts from 24
        if let Some(drum_override) = self.override_drums.get(id).copied().flatten() {
            drum.instrument = drum_override.0;
            drum.key = drum_override.1;
        }
        Some(drum)
    }
}

impl Default for MidiInstrumentSet {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! midi_ins {
    ($name: literal, $ins: ident, $octave: literal, $short_name: literal) => {
        MidiInstrument {
            name: $name,
            instrument: Instrument::$ins,
            octave: $octave,
            short_name: Some($short_name),
        }
    };
    ($name: literal, $ins: ident, $octave: literal) => {
        MidiInstrument {
            name: $name,
            instrument: Instrument::$ins,
            octave: $octave,
            short_name: None,
        }
    };
}

pub const MIDI_INSTRUMENTS: [MidiInstrument; 128] = [
    midi_ins!("Acoustic Grand Piano", Harp, 0, "Piano 1"),
    midi_ins!("Bright Acoustic Piano", Pling, 0, "Piano 2"),
    midi_ins!("Electric Grand Piano", Pling, 0, "Piano 3"),
    midi_ins!("Honky-tonk Piano", Pling, 0, "Honky-tonk"),
    midi_ins!("Electric Piano 1", Harp, 0, "E.Piano 1"),
    midi_ins!("Electric Piano 2", Harp, 0, "E.Piano 2"),
    midi_ins!("Harpsichord", Guitar, 1),
    midi_ins!("Clavinet", Banjo, 0),
    // Chromatic Percussion
    midi_ins!("Celesta", Bell, -2),
    midi_ins!("Glockenspiel", Bell, -2),
    midi_ins!("Music Box", Bell, -2),
    midi_ins!("Vibraphone", IronXylophone, 0),
    midi_ins!("Marimba", IronXylophone, 0),
    midi_ins!("Xylophone", Xylophone, -2),
    midi_ins!("Tubular Bells", Bell, -2, "TubularBells"),
    midi_ins!("Dulcimer", Guitar, 1),
    // Organ
    midi_ins!("Drawbar Organ", Flute, -1, "Organ 1"),
    midi_ins!("Percussive Organ", IronXylophone, 10, "Organ 2"),
    midi_ins!("Rock Organ", Flute, -1),
    midi_ins!("Church Organ", Flute, -1),
    midi_ins!("Reed Organ", Flute, -1),
    midi_ins!("Accordion", Flute, -1),
    midi_ins!("Harmonica", Flute, -1),
    midi_ins!("Bandoneon", Flute, -1),
    // Guitar
    midi_ins!("Acoustic Guitar (nylon)", Guitar, 1, "Nylon-str.Gt"),
    midi_ins!("Acoustic Guitar (steel)", Guitar, 1, "Steel-str.Gt"),
    midi_ins!("Electric Guitar (jazz)", Harp, 0, "Jazz Guitar"),
    midi_ins!("Electric Guitar (clean)", Guitar, 1, "Clean Guitar"),
    midi_ins!("Electric Guitar (muted)", DoubleBass, 2, "Muted Guitar"),
    midi_ins!("Overdriven Guitar", Didgeridoo, 2, "Overdrive Gt"),
    midi_ins!("Distortion Guitar", Guitar, 1, "DistortionGt"),
    midi_ins!("Guitar Harmonics", Guitar, 3, "Gt.Harmonics"),
    // Bass
    midi_ins!("Acoustic Bass", DoubleBass, 2, "AcousticBass"),
    midi_ins!("Electric Bass (finger)", DoubleBass, 2, "FingeredBass"),
    midi_ins!("Electric Bass (pick)", DoubleBass, 2, "Picked Bass"),
    midi_ins!("Fretless Bass", DoubleBass, 2, "FretlessBass"),
    midi_ins!("Slap Bass 1", Guitar, 1),
    midi_ins!("Slap Bass 2", Guitar, 1),
    midi_ins!("Synth Bass 1", DoubleBass, 2),
    midi_ins!("Synth Bass 2", Pling, 0),
    // Strings
    midi_ins!("Violin", Flute, -1),
    midi_ins!("Viola", Flute, -1),
    midi_ins!("Cello", Flute, -1),
    midi_ins!("Contrabass", Didgeridoo, 2),
    midi_ins!("Tremolo Strings", Flute, -1, "Tremolo Str."),
    midi_ins!("Pizzicato Strings", DoubleBass, 2, "Pizzicato"),
    midi_ins!("Orchestral Harp", Harp, 0, "Harp"),
    midi_ins!("Timpani", SnareDrum, 0),
    // Ensemble
    midi_ins!("String Ensemble 1", Flute, -1, "Strings 1"),
    midi_ins!("String Ensemble 2", Flute, -1, "Strings 2"),
    midi_ins!("Synth Strings 1", Flute, -1, "Syn.Strings1"),
    midi_ins!("Synth Strings 2", Flute, -1, "Syn.Strings2"),
    midi_ins!("Choir Aahs", Flute, -1),
    midi_ins!("Voice Oohs", Flute, -1),
    midi_ins!("Synth Voice", Flute, -1),
    midi_ins!("Orchestra Hit", SnareDrum, 0, "OrchestraHit"),
    // Brass
    midi_ins!("Trumpet", Flute, -1),
    midi_ins!("Trombone", Didgeridoo, 2),
    midi_ins!("Tuba", Didgeridoo, 2),
    midi_ins!("Muted Trumpet", Didgeridoo, 2, "MutedTrumpet"),
    midi_ins!("French Horn", Flute, -1),
    midi_ins!("Brass Section", Flute, -1, "Brass"),
    midi_ins!("Synth Brass 1", Flute, -1, "Synth Brass1"),
    midi_ins!("Synth Brass 2", Flute, -1, "Synth Brass2"),
    // Reed
    midi_ins!("Soprano Sax", Flute, -1),
    midi_ins!("Alto Sax", Flute, -1),
    midi_ins!("Tenor Sax", Flute, -1),
    midi_ins!("Baritone Sax", Flute, -1),
    midi_ins!("Oboe", Flute, -1),
    midi_ins!("English Horn", Flute, -1),
    midi_ins!("Bassoon", Flute, -1),
    midi_ins!("Clarinet", Flute, -1),
    // Pipe
    midi_ins!("Piccolo", Flute, -1),
    midi_ins!("Flute", Flute, -1),
    midi_ins!("Recorder", Flute, -1),
    midi_ins!("Pan Flute", Flute, -1),
    midi_ins!("Blown Bottle", Flute, -1),
    midi_ins!("Shakuhachi", Flute, -1),
    midi_ins!("Whistle", Flute, -1),
    midi_ins!("Ocarina", Flute, -1),
    // Synth Lead
    midi_ins!("Lead 1 (square)", Bit, 0, "Square Lead"),
    midi_ins!("Lead 2 (sawtooth)", Flute, -1, "Saw Lead"),
    midi_ins!("Lead 3 (calliope)", Flute, -1, "Calliope"),
    midi_ins!("Lead 4 (chiff)", Flute, -1, "Chiff Lead"),
    midi_ins!("Lead 5 (charang)", Guitar, 1, "Charang"),
    midi_ins!("Lead 6 (voice)", Flute, -1, "Voice Lead"),
    midi_ins!("Lead 7 (fifths)", Flute, -1, "Fifths Lead"),
    midi_ins!("Lead 8 (bass + lead)", DoubleBass, 2, "Bass+Lead"),
    // Synth Pad
    midi_ins!("Pad 1 (new age)", Bell, -2, "Fantasia"),
    midi_ins!("Pad 2 (warm)", Flute, -1, "Warm Pad"),
    midi_ins!("Pad 3 (polysynth)", Flute, -1, "Polysynth"),
    midi_ins!("Pad 4 (choir)", Flute, -1, "Space Choir"),
    midi_ins!("Pad 5 (bowed)", Flute, -1, "Bowed Glass"),
    midi_ins!("Pad 6 (metallic)", Flute, -1, "Metal Pad"),
    midi_ins!("Pad 7 (halo)", Flute, -1, "Halo Pad"),
    midi_ins!("Pad 8 (sweep)", Chime, -2, "Sweep Pad"),
    // Synth Effects
    midi_ins!("FX 1 (rain)", Chime, -2, "Rain Drop"),
    midi_ins!("FX 2 (soundtrack)", Flute, -1, "Soundtrack"),
    midi_ins!("FX 3 (crystal)", Chime, -2, "Crystal"),
    midi_ins!("FX 4 (atmosphere)", Guitar, 1, "Atmosphere"),
    midi_ins!("FX 5 (brightness)", Pling, 0, "Brightness"),
    midi_ins!("FX 6 (goblins)", Flute, -1, "Goblins"),
    midi_ins!("FX 7 (echoes)", Flute, -1, "Echoes"),
    midi_ins!("FX 8 (sci-fi)", Guitar, 1, "SF"),
    // Ethnic
    midi_ins!("Sitar", Banjo, 0),
    midi_ins!("Banjo", Banjo, 0),
    midi_ins!("Shamisen", Banjo, 0),
    midi_ins!("Koto", Guitar, 1),
    midi_ins!("Kalimba", IronXylophone, 0),
    midi_ins!("Bag pipe", Flute, -1),
    midi_ins!("Fiddle", Flute, -1),
    midi_ins!("Shanai", Flute, -1),
    // Percussive
    midi_ins!("Tinkle Bell", Chime, -2),
    midi_ins!("Agogo", CowBell, -1),
    midi_ins!("Steel Drums", IronXylophone, 0),
    midi_ins!("Woodblock", Xylophone, -2),
    midi_ins!("Taiko Drum", BassDrum, 0),
    midi_ins!("Melodic Tom", BassDrum, 0),
    midi_ins!("Synth Drum", BassDrum, 0),
    midi_ins!("Reverse Cymbal", SnareDrum, 0, "Reverse Cym."),
    // Sound effects
    midi_ins!("Guitar Fret Noise", Click, 1, "Gt FretNoise"),
    midi_ins!("Breath Noise", Flute, -1),
    midi_ins!("Seashore", Chime, -2),
    midi_ins!("Bird Tweet", Flute, -1, "Bird"),
    midi_ins!("Telephone Ring", Bell, 2, "Telephone"),
    midi_ins!("Helicopter", BassDrum, 0),
    midi_ins!("Applause", SnareDrum, 0),
    midi_ins!("Gunshot", SnareDrum, 0),
];

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub struct MidiPercussion {
    pub name: &'static str,
    pub instrument: Instrument,
    pub key: i8,
}

macro_rules! midi_drum {
    ($name: literal, $ins: ident, $key: literal) => {
        MidiPercussion {
            name: $name,
            instrument: Instrument::$ins,
            key: $key,
        }
    };
}

pub const MIDI_DRUMS: [MidiPercussion; 65] = [
    midi_drum!("[GS] Concert SD", SnareDrum, 12),
    midi_drum!("[GS] Snare Roll", SnareDrum, 5),
    midi_drum!("[GS] Finger Snap", Click, 18),
    midi_drum!("High Q", SnareDrum, 2),
    midi_drum!("Slap", SnareDrum, 9),
    midi_drum!("Scratch Push", Click, 6),
    midi_drum!("Scratch Pull", Click, 2),
    midi_drum!("Sticks", Click, 13),
    midi_drum!("Square Click", Click, 9),
    midi_drum!("Metronome Click", Click, 15),
    midi_drum!("Metronome Bell", Chime, 18),
    midi_drum!("Acoustic Bass Drum", BassDrum, 4),
    midi_drum!("Electric Bass Drum", BassDrum, 8),
    midi_drum!("Side Stick", Click, 8),
    midi_drum!("Acoustic Snare", SnareDrum, 15),
    midi_drum!("Hand Clap", SnareDrum, 19),
    midi_drum!("Electric Snare", SnareDrum, 16),
    midi_drum!("Low Floor Tom", BassDrum, 6),
    midi_drum!("Closed Hi-hat", Click, 21),
    midi_drum!("High Floor Tom", BassDrum, 9),
    midi_drum!("Pedal Hi-hat", Click, 23),
    midi_drum!("Low Tom", BassDrum, 14),
    midi_drum!("Open Hi-hat", SnareDrum, 22),
    midi_drum!("Low-Mid Tom", BassDrum, 17),
    midi_drum!("High-Mid Tom", BassDrum, 20),
    midi_drum!("Crash Cymbal 1", SnareDrum, 20),
    midi_drum!("High Tom", BassDrum, 23),
    midi_drum!("Ride Cymbal 1", SnareDrum, 17),
    midi_drum!("Chinese Cymbal", SnareDrum, 14),
    midi_drum!("Ride Bell", Bell, 17),
    midi_drum!("Tambourine", SnareDrum, 23),
    midi_drum!("Splash Cymbal", SnareDrum, 18),
    midi_drum!("Cowbell", CowBell, 6),
    midi_drum!("Crash Cymbal 2", SnareDrum, 21),
    midi_drum!("Vibraslap", Click, 17),
    midi_drum!("Ride Cymbal 2", SnareDrum, 24),
    midi_drum!("High Bongo", CowBell, 16),
    midi_drum!("Low Bongo", CowBell, 9),
    midi_drum!("Mute Hi Conga", Click, -3),
    midi_drum!("Open Hi Conga", CowBell, -1),
    midi_drum!("Low Conga", CowBell, -9),
    midi_drum!("High Timbale", CowBell, 5),
    midi_drum!("Low Timbale", CowBell, -4),
    midi_drum!("High Agogo", Xylophone, 12),
    midi_drum!("Low Agogo", Xylophone, 5),
    midi_drum!("Cabasa", Click, 35),
    midi_drum!("Maracas", Click, 32),
    midi_drum!("Short Whistle", Flute, 34),
    midi_drum!("Long Whistle", Flute, 33),
    midi_drum!("Short Guiro", Click, 19),
    midi_drum!("Long Guiro", Click, 20),
    midi_drum!("Claves", Xylophone, 19),
    midi_drum!("Hi Wood Block", Xylophone, 7),
    midi_drum!("Low Wood Block", Xylophone, 1),
    midi_drum!("Mute Cuica", Didgeridoo, 22),
    midi_drum!("Open Cuica", Didgeridoo, 13),
    midi_drum!("Mute Triangle", Bell, 19),
    midi_drum!("Open Triangle", Chime, 19),
    midi_drum!("Shaker", Click, 36),
    midi_drum!("Jingle Bell", Bell, 21),
    midi_drum!("Bell Tree", Chime, 17),
    midi_drum!("Castanets", Xylophone, 15),
    midi_drum!("Mute Surdo", BassDrum, 12),
    midi_drum!("Open Surdo", BassDrum, 7),
    midi_drum!("[GS] Applause", SnareDrum, 10),
];

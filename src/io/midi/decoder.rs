use std::collections::{BTreeSet, HashMap};

use midly::{Format, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use thiserror::Error;

use crate::{
    Nbs, Tick,
    instrument::{CustomInstrument, TEMPO_CHANGER},
    io::midi::midi_instrument::{MIDI_DRUMS, MIDI_INSTRUMENTS, MidiInstrument},
    noteblock::Note,
};

#[derive(Debug, Error)]
pub enum MidiDecodeError {
    #[error("Unsupported MIDI format: 2 (sequential)")]
    UnsupportedFormat2,
    #[error("Failed to parse MIDI data: {0}")]
    MidiParseError(#[from] midly::Error),
    #[error("Failed to decode MIDI string data: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("MIDI pitch value out of range: {0}")]
    PitchOverflow(f32),
}

#[derive(Debug, Clone, Default)]
struct Track {
    name: Option<String>,
    height: usize,
    notes: HashMap<Tick, Vec<Note>>,
}

pub fn decode_from_midi(midi_bytes: &[u8]) -> Result<Nbs, MidiDecodeError> {
    let smf = Smf::parse(midi_bytes)?;
    match smf.header.format {
        Format::Sequential => return Err(MidiDecodeError::UnsupportedFormat2),
        _ => {}
    }
    let mut nbs = Nbs::new();
    let mut instrument_per_channels = [None; 16];
    let resolution = match smf.header.timing {
        Timing::Metrical(t) => t.as_int() as f32,
        Timing::Timecode(_, _) => unimplemented!("Timecode timing is not supported"), //TODO: support
    };
    let mut tracks = vec![Track::default(); smf.tracks.len()];
    let mut intervals = BTreeSet::new();
    let mut tempo_changers = Vec::new();

    for (track_index, track) in smf.tracks.iter().enumerate() {
        let mut pos = 0;
        let mut prev_pos;
        for event in track {
            let delta = event.delta.as_int();
            prev_pos = pos;
            pos += delta;
            match event.kind {
                TrackEventKind::Meta(message) => match message {
                    MetaMessage::Copyright(copyright_data) => {
                        nbs.header.song_info.description =
                            String::from_utf8(copyright_data.to_vec()).unwrap_or_default();
                    }
                    MetaMessage::TrackName(name_data) => {
                        let name = String::from_utf8(name_data.to_vec()).unwrap_or_default();
                        if name.is_empty() {
                            tracks[track_index].name = Some(name);
                        }
                    }
                    MetaMessage::Tempo(tempo) => {
                        if pos > prev_pos {
                            intervals.insert(pos - prev_pos);
                        }
                        let tempo = 1_000_000.0 / tempo.as_int() as f32 * resolution; // bps
                        tempo_changers.push((pos, tempo));
                    }
                    MetaMessage::KeySignature(_, _) => {
                        //TODO: Now the midly crate has a bug that it doesn't parse an invalid length of key signature meta message, so we wait for the bug to be fixed
                        //TODO: If the bug isn't fixed, we need to patch the midly crate or wrap the parsing logic...
                    }
                    _ => {}
                },
                TrackEventKind::Midi { channel, message } => match message {
                    MidiMessage::NoteOn { key: note, vel } => {
                        if pos > prev_pos {
                            intervals.insert(pos - prev_pos);
                        }
                        let volume = (vel.as_int() as f32 / 1.27).ceil() as u8;
                        let note_entry = if channel == 9 {
                            if let Some(drum) = MIDI_DRUMS.get(note.as_int() as usize + 24) {
                                Some((drum.instrument, drum.key as i16 + 33, volume))
                            } else {
                                None
                            }
                        } else {
                            let instrument = instrument_per_channels[channel.as_int() as usize];
                            if let Some(instrument) = instrument {
                                let MidiInstrument {
                                    instrument, octave, ..
                                } = instrument;
                                let key = note.as_int() as i16 - 21 + octave as i16 * 12;
                                if volume == 0 {
                                    None
                                } else {
                                    Some((instrument, key, volume))
                                }
                            } else {
                                None
                            }
                        };
                        if let Some((instrument, key, volume)) = note_entry {
                            let key = key.clamp(0, 87) as u8;
                            let note = Note {
                                instrument,
                                key,
                                volume,
                                ..Default::default()
                            };
                            let notes = tracks[track_index].notes.entry(pos).or_default();
                            notes.push(note);
                            let note_count = notes.len();
                            tracks[track_index].height = tracks[track_index].height.max(note_count);
                        }
                    }
                    MidiMessage::ProgramChange { program } => {
                        let program = program.as_int() as usize;
                        instrument_per_channels[channel.as_int() as usize] =
                            MIDI_INSTRUMENTS.get(program).copied();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    let interval_gcd = intervals.into_iter().rev().reduce(gcd).unwrap_or(1);
    nbs.header.song_meta.tempo = 2.0 * resolution / interval_gcd as f32;

    let mut layer_base_index = 0;
    for track in tracks {
        for (tick, notes) in track.notes {
            for (layer, note) in notes.into_iter().enumerate() {
                nbs.note_blocks.place_note(
                    tick / interval_gcd,
                    (layer_base_index + layer) as u16,
                    note,
                );
            }
        }
        for layer in 0..track.height {
            nbs.note_blocks
                .layer_mut((layer_base_index + layer) as u16)
                .unwrap()
                .name = format!(
                "{} - #{}",
                track.name.as_deref().unwrap_or("Unnamed"),
                layer + 1
            );
        }
        layer_base_index += track.height;
    }
    if !tempo_changers.is_empty() {
        let instrument = nbs
            .instrument_set
            .push(CustomInstrument {
                name: TEMPO_CHANGER.to_string(),
                ..Default::default()
            })
            .unwrap();
        let mut tempo_changer_layer = nbs.note_blocks.layer_count();
        for (tick, tempo) in tempo_changers {
            let pitch = tempo / interval_gcd as f32 * 15.0;
            if 32767.0 < pitch {
                return Err(MidiDecodeError::PitchOverflow(pitch));
            }
            let pitch = pitch as i16;
            let note = Note {
                instrument,
                pitch,
                ..Default::default()
            };
            nbs.note_blocks
                .place_note(tick / interval_gcd, tempo_changer_layer, note);
            tempo_changer_layer += 1;
        }
    }

    nbs.header.song_meta.length = nbs.note_blocks.ticks_len() as u16;
    nbs.header.song_meta.layers = nbs.note_blocks.layer_count();

    Ok(nbs)
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

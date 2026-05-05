use std::io::{ErrorKind, Read};

use byteorder::{LittleEndian, ReadBytesExt};

use crate::{
    instrument::Instrument,
    io::NbsIOError,
    nbs::{
        Nbs, NbsVersion,
        custom_instrument::{CustomInstrument, CustomInstruments},
        header::{Header, Looping},
        noteblock::{Note, NoteBlocks},
    },
    nbsver_required,
};

pub trait ReadStringExt: byteorder::ReadBytesExt {
    fn read_string_len_i32<T: byteorder::ByteOrder>(&mut self) -> Result<String, std::io::Error> {
        let len = self.read_i32::<T>()?;
        let mut buf = vec![0; len as usize];
        self.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }
}

impl<R: ReadBytesExt + ?Sized> ReadStringExt for R {}

pub fn read_nbs(reader: &mut impl Read) -> Result<Nbs, NbsIOError> {
    let (version, header) = read_header(reader)?;
    let note_blocks = read_note_blocks(reader, version)?;
    let note_blocks = read_layers(reader, version, header.song_meta.layers, note_blocks)?;
    let custom_instruments = read_custom_instruments(reader, header.song_meta.vanilla_instruments)?;
    Ok(Nbs {
        version,
        header,
        note_blocks,
        custom_instruments,
    })
}

pub fn read_header(reader: &mut impl Read) -> Result<(NbsVersion, Header), NbsIOError> {
    let mut header = Header::default();

    let classic_song_len = reader.read_u16::<LittleEndian>()?;
    let version = match classic_song_len {
        0 => NbsVersion::try_from(reader.read_u8()?).map_err(NbsIOError::UnsupportedVersion)?,
        _ => NbsVersion::Classic,
    };
    header.song_meta.vanilla_instruments =
        nbsver_required!(version > Classic, reader.read_u8()?, 10);
    header.song_meta.length = nbsver_required!(
        version >= V3,
        reader.read_u16::<LittleEndian>()?,
        classic_song_len
    );
    header.song_meta.layers = reader.read_u16::<LittleEndian>()?;
    header.song_info.name = reader.read_string_len_i32::<LittleEndian>()?;
    header.song_info.author = reader.read_string_len_i32::<LittleEndian>()?;
    header.song_info.original_author = reader.read_string_len_i32::<LittleEndian>()?;
    header.song_info.description = reader.read_string_len_i32::<LittleEndian>()?;
    header.song_meta.tempo = reader.read_i16::<LittleEndian>()? as f32 / 100.0;
    header.auto_saving.enabled = reader.read_u8()? != 0;
    header.auto_saving.duration = reader.read_u8()?;
    header.song_meta.time_signature = reader.read_u8()?;
    header.song_stats.minutes_spent = reader.read_u32::<LittleEndian>()?;
    header.song_stats.left_clicks = reader.read_u32::<LittleEndian>()?;
    header.song_stats.right_clicks = reader.read_u32::<LittleEndian>()?;
    header.song_stats.note_blocks_added = reader.read_u32::<LittleEndian>()?;
    header.song_stats.note_blocks_removed = reader.read_u32::<LittleEndian>()?;
    header.midi_schematic_file_name = reader.read_string_len_i32::<LittleEndian>()?;
    header.looping = nbsver_required!(
        version >= V4,
        match (
            reader.read_u8()?,
            reader.read_u8()?,
            reader.read_u16::<LittleEndian>()?,
        ) {
            // loop == 0 means no looping
            (0, _, _) => Looping::NoLooping,
            // count == 0 means infinite looping
            (_, 0, loop_start_tick) => Looping::Infinite { loop_start_tick },
            // otherwise, we have finite looping
            (_, count, loop_start_tick) => Looping::Finite {
                count,
                loop_start_tick,
            },
        },
        Looping::NoLooping
    );
    Ok((version, header))
}

fn read_note_blocks(reader: &mut impl Read, version: NbsVersion) -> Result<NoteBlocks, NbsIOError> {
    let mut note_blocks = NoteBlocks::new();
    let mut tick = u32::MAX;
    loop {
        let jump_ticks = reader.read_u16::<LittleEndian>()?;
        if jump_ticks == 0 {
            break;
        }
        tick = tick.wrapping_add(jump_ticks as u32);
        let mut layer = u16::MAX;
        loop {
            let jump_layers = reader.read_u16::<LittleEndian>()?;
            if jump_layers == 0 {
                break;
            }
            layer = layer.wrapping_add(jump_layers);
            let note = Note {
                instrument: Instrument(reader.read_u8()?),
                key: reader.read_u8()?,
                volume: nbsver_required!(version >= V4, reader.read_u8()?, 100),
                panning: nbsver_required!(version >= V4, reader.read_u8()?, 0),
                pitch: nbsver_required!(version >= V4, reader.read_i16::<LittleEndian>()?, 0),
            };
            note_blocks.place_note(tick, layer, note);
        }
    }
    Ok(note_blocks)
}

fn read_layers(
    reader: &mut impl Read,
    version: NbsVersion,
    layer_count: u16,
    mut note_blocks: NoteBlocks,
) -> Result<NoteBlocks, NbsIOError> {
    note_blocks.extend_layers(layer_count);
    for i in 0..layer_count {
        let name = match reader.read_string_len_i32::<LittleEndian>() {
            Ok(name) => name,
            // No layers part, just return early
            Err(e) if e.kind() == ErrorKind::UnexpectedEof && i == 0 => {
                return Ok(note_blocks);
            }
            Err(e) => return Err(e.into()),
        };
        // We already extended the layers in the note blocks
        let layer = note_blocks.layer_mut(i).unwrap();
        layer.name = name;
        layer.lock = nbsver_required!(version >= V4, reader.read_u8()? != 0, false);
        layer.volume = reader.read_u8()?;
        layer.panning = nbsver_required!(version >= V2, reader.read_u8()?, 0);
    }
    Ok(note_blocks)
}

fn read_custom_instruments(
    reader: &mut impl Read,
    vanilla_instruments: u8,
) -> Result<CustomInstruments, NbsIOError> {
    let mut custom_instruments = CustomInstruments::new(vanilla_instruments);
    let custom_instrument_count = match reader.read_u8() {
        Ok(count) => count,
        // No custom instruments part, just return early
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {
            return Ok(custom_instruments);
        }
        Err(e) => return Err(e.into()),
    };
    for _ in 0..custom_instrument_count {
        let custom_instrument = CustomInstrument {
            name: reader.read_string_len_i32::<LittleEndian>()?,
            file_name: reader.read_string_len_i32::<LittleEndian>()?,
            key: reader.read_u8()?,
            press_piano_key: reader.read_u8()? != 0,
        };
        custom_instruments.push(custom_instrument).unwrap();
    }
    Ok(custom_instruments)
}

use std::io::Write;

use byteorder::{LittleEndian, WriteBytesExt};

use crate::{
    Nbs, NbsIOError, instrument::InstrumentSet, nbs::NbsVersion, nbsver_required,
    noteblock::NoteBlocks,
};

pub trait WriteStringExt: byteorder::WriteBytesExt {
    fn write_string_len_i32<T: byteorder::ByteOrder>(
        &mut self,
        s: impl AsRef<str>,
    ) -> Result<(), std::io::Error> {
        let s = s.as_ref();
        let len = s.len() as i32;
        self.write_i32::<T>(len)?;
        self.write_all(s.as_bytes())?;
        Ok(())
    }
}

impl<R: WriteBytesExt + ?Sized> WriteStringExt for R {}

/// Writes the given NBS to the writer.
/// This function trusts that the fields in the header are correctly set, and writes these fields without calculating them from the note blocks and custom instruments.
/// This function is faster than `write_nbs`, but may produce invalid NBS if the header fields are not correctly set.
/// Providing some NBS data structures from this crate such as `NoteBlocks` and `CustomInstruments` to the header fields without modification may cause invalid NBS, so be careful when using this function.
#[allow(unused)]
fn write_nbs_trusted_header(writer: &mut impl Write, nbs: &Nbs) -> Result<(), NbsIOError> {
    let version = nbs.version;
    write_header(writer, version, nbs, true)?;
    write_note_blocks_and_layers(writer, version, &nbs.note_blocks)?;
    write_custom_instruments(writer, &nbs.instrument_set)?;
    Ok(())
}

/// Writes the given NBS to the writer. This function calculating the some fields in the header from the note blocks and custom instruments, and writes these calculated values.
pub fn write_nbs(writer: &mut impl Write, nbs: &Nbs) -> Result<(), NbsIOError> {
    let version = nbs.version;
    write_header(writer, version, nbs, false)?;
    write_note_blocks_and_layers(writer, version, &nbs.note_blocks)?;
    write_custom_instruments(writer, &nbs.instrument_set)?;
    Ok(())
}

fn write_header(
    writer: &mut impl Write,
    version: NbsVersion,
    nbs: &Nbs,
    trusted: bool,
) -> Result<(), NbsIOError> {
    let header = &nbs.header;
    let vanilla_instruments = if trusted {
        header.song_meta.vanilla_instruments
    } else {
        nbs.instrument_set.vanilla_instrument_count()
    };
    let song_length = if trusted {
        header.song_meta.length
    } else {
        nbs.note_blocks.ticks_len() as u16
    };
    let layer_count = if trusted {
        header.song_meta.layers
    } else {
        nbs.note_blocks.layer_count()
    };
    nbsver_required!(
        version > Classic,
        {
            writer.write_u16::<LittleEndian>(0)?;
            writer.write_u8(version as u8)?;
            writer.write_u8(vanilla_instruments)?;
        },
        {
            writer.write_u16::<LittleEndian>(song_length)?;
        }
    );
    nbsver_required!(
        version >= V3,
        writer.write_u16::<LittleEndian>(song_length)?
    );
    writer.write_u16::<LittleEndian>(layer_count)?;
    writer.write_string_len_i32::<LittleEndian>(&header.song_info.name)?;
    writer.write_string_len_i32::<LittleEndian>(&header.song_info.author)?;
    writer.write_string_len_i32::<LittleEndian>(&header.song_info.original_author)?;
    writer.write_string_len_i32::<LittleEndian>(&header.song_info.description)?;
    writer.write_u16::<LittleEndian>((header.song_meta.tempo * 100.0) as u16)?;
    writer.write_u8(header.editor_info.auto_saving.enabled.into())?;
    writer.write_u8(header.editor_info.auto_saving.duration)?;
    writer.write_u8(header.editor_info.time_signature)?;
    writer.write_u32::<LittleEndian>(header.song_stats.minutes_spent)?;
    writer.write_u32::<LittleEndian>(header.song_stats.left_clicks)?;
    writer.write_u32::<LittleEndian>(header.song_stats.right_clicks)?;
    writer.write_u32::<LittleEndian>(header.song_stats.note_blocks_added)?;
    writer.write_u32::<LittleEndian>(header.song_stats.note_blocks_removed)?;
    writer.write_string_len_i32::<LittleEndian>(&header.editor_info.midi_schematic_file_name)?;
    nbsver_required!(version >= V4, {
        writer.write_u8(header.song_meta.looping.enabled.into())?;
        writer.write_u8(header.song_meta.looping.count.map(|c| c.get()).unwrap_or(0))?;
        writer.write_u16::<LittleEndian>(header.song_meta.looping.start_tick)?;
    });
    Ok(())
}

fn write_note_blocks_and_layers(
    writer: &mut impl Write,
    version: NbsVersion,
    note_blocks: &NoteBlocks,
) -> Result<(), NbsIOError> {
    let mut tick = u32::MAX;
    for &t in note_blocks.ticks() {
        writer.write_u16::<LittleEndian>((t.wrapping_sub(tick)) as u16)?;
        tick = t;
        let mut layer = u16::MAX;
        for (l, note) in note_blocks.notes_at_tick(tick).unwrap() {
            let l = *l;
            writer.write_u16::<LittleEndian>(l.wrapping_sub(layer))?;
            layer = l;
            writer.write_u8(note.instrument.0)?;
            writer.write_u8(note.key)?;
            nbsver_required!(version >= V4, writer.write_u8(note.volume)?);
            nbsver_required!(version >= V4, writer.write_u8(note.panning)?);
            nbsver_required!(version >= V4, writer.write_i16::<LittleEndian>(note.pitch)?);
        }
        writer.write_u16::<LittleEndian>(0)?;
    }
    writer.write_u16::<LittleEndian>(0)?;
    for layer in note_blocks.layers() {
        writer.write_string_len_i32::<LittleEndian>(&layer.name)?;
        nbsver_required!(version >= V4, writer.write_u8(layer.lock.into())?);
        writer.write_u8(layer.volume)?;
        nbsver_required!(version >= V2, writer.write_u8(layer.panning)?);
    }
    Ok(())
}

fn write_custom_instruments(
    writer: &mut impl Write,
    instrument_set: &InstrumentSet,
) -> Result<(), NbsIOError> {
    writer.write_u8(instrument_set.custom_instrument_count())?;
    for custom_instrument in instrument_set.as_slice() {
        writer.write_string_len_i32::<LittleEndian>(&custom_instrument.name)?;
        writer.write_string_len_i32::<LittleEndian>(&custom_instrument.file_name)?;
        writer.write_u8(custom_instrument.key)?;
        writer.write_u8(custom_instrument.press_piano_key.into())?;
    }
    Ok(())
}

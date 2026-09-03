use std::{borrow::Borrow, collections::VecDeque, sync::Arc};

use crate::{
    Nbs, Tick,
    audio::{NoteWeight, TempoMap},
    noteblock::{Layer, Note},
};

#[derive(Debug, Clone, Copy)]
pub enum NoteStreamEvent {
    NotePlay {
        note: Note,
        weight: NoteWeight,
    },
    TempoChange(f32),
    /// This event indicates that all events that occur at this tick have been processed, and it is necessary to advance the time until the next tick.
    TickAdvance,
    EndOfStream,
}

pub trait NoteStream {
    fn next_event(&mut self) -> NoteStreamEvent;
    fn default_tempo(&self) -> f32;
    fn clone(&self) -> Option<Self>
    where
        Self: Sized;
}

#[derive(Debug, Clone)]
pub struct NbsStream<T: Borrow<Nbs> + Clone> {
    nbs: T,
    tempo_map: Arc<TempoMap>,
    tick: Tick,
    loop_count: u8,
    queued_event: VecDeque<NoteStreamEvent>,
}

impl<T: Borrow<Nbs> + Clone> NbsStream<T> {
    pub fn new(nbs: T) -> Self {
        let tempo_map = Arc::new(TempoMap::from_nbs(nbs.borrow()));
        NbsStream {
            nbs,
            tempo_map,
            tick: 0,
            loop_count: 0,
            queued_event: VecDeque::new(),
        }
    }
}

impl<T: Borrow<Nbs> + Clone> NoteStream for NbsStream<T> {
    fn next_event(&mut self) -> NoteStreamEvent {
        if let Some(event) = self.queued_event.pop_front() {
            return event;
        }
        let nbs = self.nbs.borrow();
        if self.tick >= nbs.note_blocks.ticks_len() {
            let looping = &nbs.header.song_meta.looping;
            if looping.enabled
                && (looping.count.is_none() || self.loop_count < looping.count.unwrap().get())
            {
                self.tick = looping.start_tick as Tick;
                self.loop_count += 1;
            } else {
                return NoteStreamEvent::EndOfStream;
            }
        }
        if self.tempo_map.is_tempo_changing_tick(self.tick) {
            let tempo = self.tempo_map.get_tempo_at(self.tick);
            self.queued_event
                .push_back(NoteStreamEvent::TempoChange(tempo));
        }
        if let Some(notes) = nbs.note_blocks.notes_at_tick(self.tick) {
            for &(layer, note) in notes {
                let mut weight = NoteWeight::default();
                nbs.note_blocks.layer(layer).map(
                    |&Layer {
                         volume, panning, ..
                     }| {
                        weight.volume = volume;
                        weight.panning = panning;
                    },
                );
                nbs.instrument_set
                    .custom_instrument(note.instrument)
                    .map(|custom_instrument| {
                        weight.key = custom_instrument.key;
                    });
                self.queued_event
                    .push_back(NoteStreamEvent::NotePlay { note, weight });
            }
        }
        self.queued_event.push_back(NoteStreamEvent::TickAdvance);
        self.tick += 1;
        if let Some(event) = self.queued_event.pop_front() {
            return event;
        } else {
            return NoteStreamEvent::TickAdvance;
        }
    }

    fn default_tempo(&self) -> f32 {
        self.nbs.borrow().header.song_meta.tempo
    }

    fn clone(&self) -> Option<Self>
    where
        Self: Sized + Clone,
    {
        Some(Clone::clone(&self))
    }
}

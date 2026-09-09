use std::{borrow::Borrow, collections::VecDeque, sync::Arc};

use crate::{
    Nbs, Tick,
    audio::{NbsEvent, NbsStream, TempoMap, note_audio::NoteWeight},
    noteblock::{Layer, Note},
};

#[derive(Debug, Clone)]
pub struct StaticNbsStream<T: Borrow<Nbs> + Clone> {
    nbs: T,
    tempo_map: Arc<TempoMap>,
    tick: Tick,
    loop_count: u8,
    queued_event: VecDeque<NbsEvent<(Note, NoteWeight)>>,
}

impl<T: Borrow<Nbs> + Clone> StaticNbsStream<T> {
    pub fn new(nbs: T) -> Self {
        let tempo_map = Arc::new(TempoMap::from_nbs(nbs.borrow()));
        StaticNbsStream {
            nbs,
            tempo_map,
            tick: 0,
            loop_count: 0,
            queued_event: VecDeque::new(),
        }
    }
}

impl<T: Borrow<Nbs> + Clone> NbsStream<(Note, NoteWeight)> for StaticNbsStream<T> {
    fn next_event(&mut self) -> NbsEvent<(Note, NoteWeight)> {
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
                return NbsEvent::EndOfStream;
            }
        }
        if self.tempo_map.is_tempo_changing_tick(self.tick) {
            let tempo = self.tempo_map.get_tempo_at(self.tick);
            self.queued_event.push_back(NbsEvent::TempoChange(tempo));
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
                    .push_back(NbsEvent::NotePlay((note, weight)));
            }
        }
        self.queued_event.push_back(NbsEvent::TickAdvance);
        self.tick += 1;
        if let Some(event) = self.queued_event.pop_front() {
            return event;
        } else {
            return NbsEvent::TickAdvance;
        }
    }

    fn default_tempo(&self) -> f32 {
        self.nbs.borrow().header.song_meta.tempo
    }
}

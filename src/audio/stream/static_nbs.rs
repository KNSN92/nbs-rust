use std::{borrow::Borrow, collections::VecDeque};

use crate::{
    Nbs, Tick,
    audio::{NbsEvent, NbsStream, TempoMap, note_audio::NoteWeight},
    noteblock::{Layer, Note},
};

#[derive(Debug)]
pub struct StaticNbsStream<T: Borrow<Nbs> + Clone> {
    nbs: T,
    state: StaticNbsStreamState,
    tempo_map: TempoMap,
    queued_event: VecDeque<NbsEvent<(Note, NoteWeight), StaticNbsStreamState>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StaticNbsStreamState {
    pub tick: Tick,
    pub tempo: f32,
    pub loop_count: u8,
}

impl<T: Borrow<Nbs> + Clone> StaticNbsStream<T> {
    pub fn new(nbs: T) -> Self {
        let tempo = nbs.borrow().header.song_meta.tempo;
        let state = StaticNbsStreamState {
            tick: 0,
            tempo,
            loop_count: 0,
        };
        let tempo_map = TempoMap::from_nbs(nbs.borrow());
        StaticNbsStream {
            nbs,
            state,
            tempo_map,
            queued_event: VecDeque::new(),
        }
    }
}

impl<T: Borrow<Nbs> + Clone> NbsStream<(Note, NoteWeight), StaticNbsStreamState>
    for StaticNbsStream<T>
{
    fn next_event(&mut self) -> NbsEvent<(Note, NoteWeight), StaticNbsStreamState> {
        if let Some(event) = self.queued_event.pop_front() {
            return event;
        }
        let nbs = self.nbs.borrow();
        if self.state.tick >= nbs.note_blocks.ticks_len() {
            let looping = &nbs.header.song_meta.looping;
            if looping.enabled
                && (looping.count.is_none() || self.state.loop_count < looping.count.unwrap().get())
            {
                self.state.tick = looping.start_tick as Tick;
                self.state.loop_count += 1;
            } else {
                return NbsEvent::EndOfStream;
            }
        }
        if self.tempo_map.is_tempo_changing_tick(self.state.tick) {
            let tempo = self.tempo_map.get_tempo_at(self.state.tick);
            self.queued_event.push_back(NbsEvent::TempoChange(tempo));
        }
        if let Some(notes) = nbs.note_blocks.notes_at_tick(self.state.tick) {
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
        self.state.tick += 1;
        self.queued_event
            .push_back(NbsEvent::CustomEvent(self.state));
        self.queued_event.push_back(NbsEvent::TickAdvance);
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

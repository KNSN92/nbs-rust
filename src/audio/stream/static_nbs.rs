use std::{borrow::Borrow, collections::VecDeque, sync::{Arc, RwLock, Weak}};

use crate::{
    Nbs, Tick,
    audio::{NbsEvent, NbsStream, TempoMap, note_audio::NoteWeight},
    noteblock::{Layer, Note},
};

#[derive(Debug, Clone)]
pub struct StaticNbsStream<T: Borrow<Nbs> + Clone> {
    nbs: T,
    state: Arc<RwLock<StaticNbsStreamState>>,
    tempo_map: Arc<TempoMap>,
    queued_event: VecDeque<NbsEvent<(Note, NoteWeight)>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StaticNbsStreamState {
    pub tick: Tick,
    pub tempo: f32,
    pub loop_count: u8,
}

// StaticNbsStreamの外部からtickなどの状態を取得するためにstateのWeak参照を内部で持つ構造体
pub struct StaticNbsStreamStateHandle {
    state: Weak<RwLock<StaticNbsStreamState>>,
    latest: StaticNbsStreamState,
}

impl StaticNbsStreamStateHandle {
    pub fn get_state(&mut self) -> StaticNbsStreamState {
        let state = self.state.upgrade()
            .map(
                |state| *state.read().unwrap()
            )
            .unwrap_or(self.latest);
        self.latest = state;
        state
    }
}

impl<T: Borrow<Nbs> + Clone> StaticNbsStream<T> {
    pub fn new(nbs: T) -> Self {
        let tempo = nbs.borrow().header.song_meta.tempo;
        let tempo_map = Arc::new(TempoMap::from_nbs(nbs.borrow()));
        StaticNbsStream {
            nbs,
            state: Arc::new(RwLock::new(StaticNbsStreamState {
                tick: 0,
                tempo,
                loop_count: 0,
            })),
            tempo_map,
            queued_event: VecDeque::new(),
        }
    }

    pub fn state_handle(&self) -> StaticNbsStreamStateHandle {
        StaticNbsStreamStateHandle {
            state: Arc::downgrade(&self.state),
            latest: StaticNbsStreamState::default(),
        }
    }
}

impl<T: Borrow<Nbs> + Clone> NbsStream<(Note, NoteWeight)> for StaticNbsStream<T> {
    fn next_event(&mut self) -> NbsEvent<(Note, NoteWeight)> {
        if let Some(event) = self.queued_event.pop_front() {
            return event;
        }
        let nbs = self.nbs.borrow();
        let mut state = *self.state.read().unwrap();
        if state.tick >= nbs.note_blocks.ticks_len() {
            let looping = &nbs.header.song_meta.looping;
            if looping.enabled
                && (looping.count.is_none() || state.loop_count < looping.count.unwrap().get())
            {
                state.tick = looping.start_tick as Tick;
                state.loop_count += 1;
            } else {
                return NbsEvent::EndOfStream;
            }
        }
        if self.tempo_map.is_tempo_changing_tick(state.tick) {
            let tempo = self.tempo_map.get_tempo_at(state.tick);
            self.queued_event.push_back(NbsEvent::TempoChange(tempo));
        }
        if let Some(notes) = nbs.note_blocks.notes_at_tick(state.tick) {
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
        state.tick += 1;
        *self.state.write().unwrap() = state;
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

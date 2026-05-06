use std::collections::HashMap;

use crate::instrument::Instrument;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Note {
    pub instrument: Instrument,
    pub key: u8,
    pub volume: u8,
    pub panning: u8,
    pub pitch: i16,
}

impl Default for Note {
    fn default() -> Self {
        Note {
            instrument: Instrument::Harp,
            key: 0,
            volume: 100,
            panning: 0,
            pitch: 0,
        }
    }
}

pub type Tick = u32;
pub type Layer = u16;
pub type NotesInTick = Vec<(Layer, Note)>;
pub type NotesInLayer = Vec<(Tick, Note)>;

#[derive(Debug)]
pub struct NoteBlocks {
    len: Tick,
    ticks: Vec<Tick>,
    layers: Vec<LayerMetadata>,
    by_tick_notes: HashMap<Tick, NotesInTick>,
    by_layer_notes: HashMap<Layer, NotesInLayer>,
}

impl NoteBlocks {
    pub fn new() -> Self {
        NoteBlocks {
            len: 0,
            ticks: Vec::new(),
            layers: Vec::new(),
            by_tick_notes: HashMap::new(),
            by_layer_notes: HashMap::new(),
        }
    }

    pub fn ticks_len(&self) -> Tick {
        self.len
    }

    pub fn layer_count(&self) -> u16 {
        self.layers.len() as u16
    }

    pub fn ticks(&self) -> &[Tick] {
        &self.ticks
    }

    pub fn layers(&self) -> &[LayerMetadata] {
        &self.layers
    }

    pub(crate) fn extend_layers(&mut self, layer: Layer) {
        while self.layers.len() <= layer as usize {
            self.by_layer_notes
                .insert(self.layers.len() as Layer, vec![]);
            self.layers.push(LayerMetadata::default());
        }
    }

    pub fn layer(&self, layer: Layer) -> Option<&LayerMetadata> {
        self.layers.get(layer as usize)
    }

    pub fn layer_mut(&mut self, layer: Layer) -> Option<&mut LayerMetadata> {
        self.layers.get_mut(layer as usize)
    }

    pub fn notes_at_tick(&self, tick: Tick) -> Option<&NotesInTick> {
        self.by_tick_notes.get(&tick)
    }

    pub fn notes_at_layer(&self, layer: Layer) -> Option<&NotesInLayer> {
        self.by_layer_notes.get(&layer)
    }

    pub fn note_at(&self, tick: Tick, layer: Layer) -> Option<&Note> {
        self.by_tick_notes.get(&tick).and_then(|notes_in_tick| {
            notes_in_tick
                .iter()
                .find(|(l, _)| *l == layer)
                .map(|(_, note)| note)
        })
    }

    pub fn place_note(&mut self, tick: Tick, layer: Layer, note: Note) {
        self.len = self.len.max(tick + 1);

        let notes = self.by_tick_notes.entry(tick).or_default();
        notes.push((layer, note));
        notes.sort_by_key(|(l, _)| *l);

        let notes = self.by_layer_notes.entry(layer).or_default();
        notes.push((tick, note));
        notes.sort_by_key(|(t, _)| *t);

        // If the tick is new, insert it into the ticks vector while keeping it sorted
        let _ = self
            .ticks
            .binary_search(&tick)
            .map_err(|i| self.ticks.insert(i, tick));

        // If the layer is new, extend the layers to accommodate it
        self.extend_layers(layer);
    }

    pub fn inner_tick_notes(&self) -> &HashMap<Tick, NotesInTick> {
        &self.by_tick_notes
    }

    pub fn inner_layer_notes(&self) -> &HashMap<Layer, NotesInLayer> {
        &self.by_layer_notes
    }
}

impl Default for NoteBlocks {
    fn default() -> Self {
        NoteBlocks::new()
    }
}

#[derive(Debug)]
pub struct LayerMetadata {
    pub name: String,
    pub lock: bool,
    pub volume: u8,
    pub panning: u8,
}

impl Default for LayerMetadata {
    fn default() -> Self {
        LayerMetadata {
            name: String::new(),
            lock: false,
            volume: 100,
            panning: 0,
        }
    }
}

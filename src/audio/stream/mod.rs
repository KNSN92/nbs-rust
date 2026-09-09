pub mod audio;
pub mod static_nbs;

#[derive(Debug, Clone, Copy)]
pub enum NbsEvent<T> {
    NotePlay(T),
    TempoChange(f32),
    NoOp,
    /// This event indicates that all events that occur at this tick have been processed, and it is necessary to advance the time until the next tick.
    TickAdvance,
    EndOfStream,
}

pub trait NbsStream<T> {
    fn next_event(&mut self) -> NbsEvent<T>;
    fn default_tempo(&self) -> f32;
}

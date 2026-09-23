pub mod audio;
pub mod custom_event;
pub mod static_nbs;

#[derive(Debug, Clone, Copy)]
pub enum NbsEvent<T, C = ()>
where
    C: Clone,
{
    NotePlay(T),
    TempoChange(f32),
    CustomEvent(C),
    NoOp,
    /// This event indicates that all events that occur at this tick have been processed, and it is necessary to advance the time until the next tick.
    TickAdvance,
    EndOfStream,
}

pub trait NbsStream<T, C = ()>
where
    C: Clone,
{
    fn next_event(&mut self) -> NbsEvent<T, C>;
    fn default_tempo(&self) -> f32;
}

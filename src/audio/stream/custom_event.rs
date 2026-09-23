use std::{
    marker::PhantomData,
    sync::{Arc, RwLock, Weak},
};

use crate::audio::{NbsEvent, NbsStream};

pub struct CustomEventHandler<C>
where
    C: Clone,
{
    last_custom_event: Weak<RwLock<Option<C>>>,
}

impl<C> CustomEventHandler<C>
where
    C: Clone,
{
    pub fn get(&self) -> Option<C> {
        if let Some(last_custom_event) = self.last_custom_event.upgrade() {
            let last_custom_event = last_custom_event.read().unwrap();
            last_custom_event.as_ref().map(|event| event.clone())
        } else {
            None
        }
    }
}

pub struct CustomEventHandlingStream<T, N, C>
where
    T: NbsStream<N, C>,
    C: Clone,
{
    _note_type: PhantomData<N>,
    stream: T,
    last_custom_event: Arc<RwLock<Option<C>>>,
}

impl<T, N, C> CustomEventHandlingStream<T, N, C>
where
    T: NbsStream<N, C>,
    C: Clone,
{
    pub fn new(stream: T) -> (Self, CustomEventHandler<C>) {
        let last_custom_event = Arc::new(RwLock::new(None));
        let handler = CustomEventHandler {
            last_custom_event: Arc::downgrade(&last_custom_event),
        };
        let stream = CustomEventHandlingStream {
            _note_type: PhantomData,
            stream,
            last_custom_event,
        };
        (stream, handler)
    }
}

impl<T, N, C> NbsStream<N, C> for CustomEventHandlingStream<T, N, C>
where
    T: NbsStream<N, C>,
    C: Clone,
{
    fn next_event(&mut self) -> crate::audio::NbsEvent<N, C> {
        let event = self.stream.next_event();
        if let NbsEvent::CustomEvent(custom_event) = &event {
            let mut last_custom_event = self.last_custom_event.write().unwrap();
            *last_custom_event = Some(custom_event.clone());
        }
        event
    }

    fn default_tempo(&self) -> f32 {
        self.stream.default_tempo()
    }
}

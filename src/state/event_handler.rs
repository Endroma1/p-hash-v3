use std::ops::{Deref, DerefMut};

use tokio_stream::{Stream, wrappers::ReceiverStream};

use crate::Progress;

pub fn create_event_bus() -> (EventHandler, EventStream) {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    (
        EventHandler { tx },
        EventStream {
            stream: Box::new(ReceiverStream::new(rx)),
        },
    )
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    FetchProgress { progress: Progress },
    ParseProgress { progress: Progress },
}

#[derive(Debug)]
pub struct EventHandler {
    tx: tokio::sync::mpsc::Sender<Event>,
}

/// Handler for reading events from app.
pub struct EventStream {
    stream: Box<dyn Stream<Item = Event> + Send + Sync + Unpin>,
}
impl Deref for EventStream {
    type Target = Box<dyn Stream<Item = Event> + Send + Sync + Unpin>;
    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}
impl DerefMut for EventStream {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.stream
    }
}

impl EventHandler {
    pub async fn send_fetch_progress(&self, progress: Progress) {
        if let Err(_) = self.tx.try_send(Event::FetchProgress { progress }) {
            tracing::warn!("Dropped sending progress {:?}. Channel is full", progress);
        };
    }
}

use sqlx::PgPool;

use super::*;
use std::{ops::Deref, sync::atomic::AtomicBool};

#[derive(Clone)]
pub struct State {
    pub inner: std::sync::Arc<InnerState>,
}
impl State {
    pub fn new(pool: PgPool) -> (Self, EventStream) {
        let (inner, event_stream) = InnerState::new(pool);
        (
            Self {
                inner: std::sync::Arc::new(inner),
            },
            event_stream,
        )
    }
}
impl Deref for State {
    type Target = InnerState;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct InnerState {
    pub is_running: AtomicBool,
    pub event_handler: EventHandler,
    pub settings: Settings,
    pub db_pool: PgPool,
}
impl InnerState {
    pub fn new(pool: PgPool) -> (Self, EventStream) {
        let (event_handler, event_stream) = create_event_bus();
        let inner_state = Self {
            is_running: AtomicBool::new(false),
            event_handler,
            settings: Settings::default(),
            db_pool: pool

        };
        (inner_state, event_stream)
    }
}

#[derive(Default)]
pub struct StateBuilder {
    progress: Progress,
    settings: Settings,
}
impl StateBuilder {
    pub fn with_progress(mut self, progress: Progress) -> Self {
        self.progress = progress;
        self
    }
    pub fn with_settings(mut self, settings: Settings) -> Self {
        self.settings = settings;
        self
    }

    pub fn build(self, pool: PgPool) -> (State, EventStream) {
        let (event_handler, event_stream) = create_event_bus();
        let inner = InnerState {
            is_running: AtomicBool::new(false),
            event_handler,
            settings: self.settings,
            db_pool: pool
        };

        (
            State {
                inner: std::sync::Arc::new(inner),
            },
            event_stream,
        )
    }
}


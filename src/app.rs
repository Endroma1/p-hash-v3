use std::sync::atomic::AtomicBool;

use anyhow::Context;
use futures::{StreamExt, executor::block_on_stream};
use sqlx::PgPool;
use tokio::task::spawn_blocking;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
    AtomicProgress, EventHandler, EventStream, Fetcher, HashingMethods, Modifications, Progress,
    Settings, create_event_bus, parse_image_rayon, parse_results,
};

pub struct App {
    is_running: AtomicBool,
    event_handler: EventHandler,
    db_pool: PgPool,
}

impl App {
    pub fn new(db_pool: PgPool) -> (Self, EventStream) {
        let (handler, stream) = create_event_bus();
        let app = Self {
            is_running: AtomicBool::default(),
            event_handler: handler,
            db_pool,
        };

        (app, stream)
    }
    // Runs modification and hashing
    // Blocks until done. Relieves thread when sending results to db
    pub async fn run(&self, settings: &Settings) -> Result<(), RunError> {
        self.is_running
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let images_n = settings.images_n;

        let stream = Fetcher::Picsum { images_n }.execute().await.unwrap();
        let modifications = Modifications::from(&settings.modifications);
        let hashing_methods = HashingMethods::from(&settings.hashing_methods);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        spawn_blocking(move || {
            let images = block_on_stream(stream);
            let results = parse_image_rayon(images, &modifications, &hashing_methods);
            for result in results {
                if tx.send(result).is_err() {
                    tracing::error!("Parse Result receiver exited before all results were parsed");
                    break;
                };
            }
        });

        let results = UnboundedReceiverStream::new(rx);

        let progress = AtomicProgress::default();
        progress.set_max(settings.expected_number_of_results());

        let progress_sender = &self.event_handler;

        let results = Box::pin(results.then(async |p| {
            let _ = &progress.increment_check();
            progress_sender
                .send_fetch_progress(Progress::from(&progress))
                .await;
            p
        }));

        parse_results(results, &self.db_pool)
            .await
            .context("Could not parse results")?;

        self.is_running.store(false, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }
    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

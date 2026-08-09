use std::sync::atomic::AtomicBool;

use anyhow::Context;
use futures::{StreamExt, executor::block_on_stream};
use sqlx::{PgPool, query};
use tokio::task::spawn_blocking;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
    AtomicProgress, EventHandler, EventStream, HashingMethods, Modifications, Progress, Settings,
    create_event_bus, parse_image_rayon, parse_results,
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
    pub async fn run<S>(&self, configure: S) -> Result<(), AppError>
    where
        S: FnOnce(&mut Settings),
    {
        let mut settings = Settings::default();
        configure(&mut settings);

        settings.validate().context("Failed to validate settings")?;

        self.is_running
            .store(true, std::sync::atomic::Ordering::Relaxed);

        let stream = settings.fetcher.execute().await.unwrap();
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
            tracing::debug!("Finished parsing images");
        });

        let results = UnboundedReceiverStream::new(rx);

        let progress = AtomicProgress::default();
        progress.set_max(settings.expected_number_of_results());

        let progress_sender = &self.event_handler;

        let results = Box::pin(results.then(async |p| {
            let _ = &progress.increment_check();
            if settings.send_events {
                progress_sender
                    .send_fetch_progress(Progress::from(&progress))
                    .await;
            }
            p
        }));

        parse_results(results, &self.db_pool)
            .await
            .context("Could not parse results")?;

        self.is_running
            .store(false, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }
    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::Relaxed)
    }
    pub async fn facts(&self) -> Result<Facts, AppError> {
        let number_of_hashes = query!("SELECT count(id) FROM hashes")
            .fetch_one(&self.db_pool)
            .await
            .context("Could not get number of hashes from db")?
            .count
            .unwrap_or_default();

        let number_of_images = query!("SELECT count(id) FROM images")
            .fetch_one(&self.db_pool)
            .await
            .context("Could not get number of images from db")?
            .count
            .unwrap_or_default();

        let number_of_modified_images = query!("SELECT count(id) FROM modified_images")
            .fetch_one(&self.db_pool)
            .await
            .context("Could not get number of images from db")?
            .count
            .unwrap_or_default();

        let running = self.is_running();

        Ok(Facts {
            number_of_hashes,
            number_of_images,
            number_of_modified_images,
            running,
        })
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Facts {
    pub number_of_hashes: i64,
    pub number_of_images: i64,
    pub number_of_modified_images: i64,
    pub running: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

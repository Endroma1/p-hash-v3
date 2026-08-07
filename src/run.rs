use anyhow::Context;
use futures::{StreamExt, executor::block_on_stream};
use tokio::task::spawn_blocking;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{AtomicProgress, Fetcher, Progress, State, parse_image_rayon, parse_results};

// Runs modification and hashing
// Blocks until done. Relieves thread when sending results to db
pub async fn run(state: State) -> Result<(), RunError> {
    state
        .is_running
        .store(true, std::sync::atomic::Ordering::Relaxed);

    let images_n = { *state.settings.images_n.lock().unwrap() };

    println!("Starting fetcher");

    let stream = Fetcher::Picsum { images_n }.execute().await.unwrap();

    let modifications = state.settings.modifications.clone().into();
    let hashing_methods = state.settings.hashing_methods.clone().into();

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    println!("Starting processor");
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
    progress.set_max(state.settings.expected_number_of_results());

    let progress_sender = &state.event_handler;

    let results = Box::pin(results.then(async |p| {
        let _ = &progress.increment_check();
        println!("{:?}", Progress::from(&progress));
        progress_sender
            .send_fetch_progress(Progress::from(&progress))
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        p
    }));

    println!("Sending results to db");
    parse_results(results, &state.db_pool)
        .await
        .context("Could not parse results")?;

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

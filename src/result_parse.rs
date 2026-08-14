use anyhow::Context;
use futures::Stream;
use sqlx::{Executor, PgPool, Postgres, Transaction, query};
use tokio_stream::StreamExt;
use uuid::Uuid;

use crate::processor::{self, Hash};

#[tracing::instrument(name = "Parsing results into db", skip(results, db_pool))]
pub async fn parse_results(
    mut results: impl Stream<Item = processor::ParseResult> + Unpin,
    run_id: Uuid,
    db_pool: &PgPool,
) -> Result<(), ResultParseError> {
    while let Some(result) = results.next().await {
        let mut transaction = db_pool
            .begin()
            .await
            .context("Could not begin transaction")?;

        let image_uuid = send_image_to_db(run_id, result.image.uuid, &result.image.name, &mut transaction)
            .await
            .context("Could not send image to db")?;

        let modified_image_uuid = send_modified_image_to_db(
            result.modification.uuid,
            &result.modification.modification.to_string(),
            image_uuid,
            &mut transaction,
        )
        .await
        .context("Could not send modified image to db")?;

        send_hashes_to_db(
            &result.hashing_method.to_string(),
            modified_image_uuid,
            result.hash,
            &mut transaction,
        )
        .await
        .context("Could not send hashes to db")?;

        transaction
            .commit()
            .await
            .context("Could not commit transaction")?;
    }
    tracing::debug!("Result parser exiting");
    Ok(())
}

#[tracing::instrument(name = "Sending image to db", skip(transaction))]
pub async fn send_image_to_db(
    run_id: Uuid,
    id: Uuid,
    name: &str,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let query = query!(
        "INSERT INTO images (id, name, run_id) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        id,
        name,
        run_id,
    );
    transaction.execute(query).await?;
    Ok(id)
}

#[tracing::instrument(name = "Sending modified_image to db", skip(transaction))]
pub async fn send_modified_image_to_db(
    id: Uuid,
    modification_name: &str,
    original_image_id: Uuid,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let query = query!(
        "INSERT INTO modified_images (id, modification, image_id) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        id,
        modification_name,
        original_image_id
    );
    transaction.execute(query).await?;

    Ok(id)
}

#[tracing::instrument(name = "Sending hash to db", skip(transaction, hash))]
pub async fn send_hashes_to_db(
    hashing_method_name: &str,
    modified_image_id: Uuid,
    hash: Hash,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    let bytes = hash.into_inner();

    let query = query!(
        "INSERT INTO hashes (id, hashing_method_name, hash, modified_image) VALUES ($1, $2, $3 ,$4)",
        id,
        hashing_method_name,
        &bytes,
        modified_image_id
    );
    transaction.execute(query).await?;

    Ok(id)
}

#[derive(Debug, thiserror::Error)]
pub enum ResultParseError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

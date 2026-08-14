use std::{path::Path, sync::LazyLock};

use claim::assert_some;
use phash::{App, Fetcher, HashingMethodType, ModificationType, fetch_picsum_image};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool, query};
use tempdir::TempDir;
use uuid::Uuid;

static LOGGER: LazyLock<()> = LazyLock::new(|| {
    tracing_subscriber::fmt::init();
});

#[tokio::test]
async fn run_creates_correct_amount_of_entries_in_database() {
    LazyLock::force(&LOGGER);

    tracing::debug!("Settings up db");
    let pool = setup_db().await;

    let images_n = 10;
    let hashing_methods = vec![HashingMethodType::Mean, HashingMethodType::Median];
    let modifications = vec![ModificationType::Blur, ModificationType::Contrast];

    let expected_number_of_hashes =
        images_n * hashing_methods.len() as u64 * modifications.len() as u64;
    let expected_number_of_modified_images = images_n * modifications.len() as u64;

    let fetcher = Fetcher::Picsum { images_n };

    let (app, _stream) = App::new(pool.clone());

    app.run(|settings| {
        settings
            .disable_event_loop()
            .fetcher(fetcher)
            .hashing_methods(hashing_methods)
            .modifications(modifications);
    })
    .await
    .unwrap();

    tracing::debug!("Fetching results from db");
    let number_of_hashes: i64 = query!("SELECT COUNT(id) FROM hashes;")
        .fetch_one(&pool)
        .await
        .unwrap()
        .count
        .unwrap();

    let number_of_modified_images: i64 = query!("SELECT COUNT(id) FROM modified_images;")
        .fetch_one(&pool)
        .await
        .unwrap()
        .count
        .unwrap();
    let number_of_images: i64 = query!("SELECT COUNT(id) FROM images;")
        .fetch_one(&pool)
        .await
        .unwrap()
        .count
        .unwrap();

    assert_eq!(
        images_n as i64, number_of_images,
        "Number of images did not match expected"
    );
    assert_eq!(
        expected_number_of_modified_images as i64, number_of_modified_images,
        "Number of modified images did not match expected"
    );
    assert_eq!(
        expected_number_of_hashes, number_of_hashes as u64,
        "Number of hashes did not match expected"
    );
}

#[tokio::test]
async fn test_local_image_fetching_works() {
    let image = fetch_picsum_image()
        .await
        .expect("Could not fetch picsum images");

    let tempdir = TempDir::new("images").unwrap();

    let id = {
        let id = Uuid::new_v4();
        let save_path = tempdir.path().join(Path::new(&id.to_string()));

        image
            .image
            .save(save_path)
            .expect("Could not save test image to tempdir");
        id
    };
    LazyLock::force(&LOGGER);

    tracing::debug!("Settings up db");
    let pool = setup_db().await;

    let hashing_methods = vec![HashingMethodType::Mean, HashingMethodType::Median];
    let modifications = vec![ModificationType::Blur, ModificationType::Contrast];

    let fetcher = Fetcher::Local {
        path: tempdir.path().to_path_buf(),
    };

    let (app, _stream) = App::new(pool.clone());

    app.run(|settings| {
        settings
            .disable_event_loop()
            .fetcher(fetcher)
            .hashing_methods(hashing_methods)
            .modifications(modifications);
    })
    .await
    .unwrap();

    let result = sqlx::query!(
        "SELECT count(id) FROM images WHERE name = $1",
        id.to_string()
    )
    .fetch_one(&pool)
    .await
    .expect("Could not execute query")
    .count;

    assert_some!(result);
}

async fn setup_db() -> PgPool {
    let mut adm_conn = PgConnection::connect("postgres://postgres:postgres@localhost/postgres")
        .await
        .unwrap();

    let id = Uuid::new_v4();

    let query = AssertSqlSafe(format!(r#"CREATE DATABASE "{}" OWNER phash;"#, id));

    sqlx::query(query)
        .execute(&mut adm_conn)
        .await
        .expect("Could not create test db");

    let test_pool = PgPool::connect(&format!("postgres://phash:password@localhost/{}", id))
        .await
        .unwrap();

    sqlx::migrate!("./migrations")
        .run(&test_pool)
        .await
        .expect("Could not migrate database");

    test_pool
}

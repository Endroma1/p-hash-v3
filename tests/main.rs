use std::{path::Path, sync::LazyLock};

use claim::assert_some;
use phash::{App, Fetcher, HashingMethodType, ModificationType, Settings, fetch_picsum_image};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool, query};
use tempdir::TempDir;
use uuid::Uuid;

static LOGGER: LazyLock<()> = LazyLock::new(|| {
    tracing_subscriber::fmt::init();
});

#[tokio::test]
async fn run_creates_correct_amount_of_entries_in_database() {
    let images_n = 10;

    let test_app = setup(Fetcher::Picsum { images_n: 10 }).await;

    let expected_number_of_hashes = images_n
        * test_app.settings.hashing_methods.len() as u64
        * test_app.settings.modifications.len() as u64;
    let expected_number_of_modified_images =
        images_n * test_app.settings.modifications.len() as u64;

    let (app, _stream) = App::new(test_app.pool.clone());
    app.run_with(test_app.settings).await.unwrap();

    tracing::debug!("Fetching results from db");
    let number_of_hashes: i64 = query!("SELECT COUNT(id) FROM hashes;")
        .fetch_one(&test_app.pool)
        .await
        .unwrap()
        .count
        .unwrap();

    let number_of_modified_images: i64 = query!("SELECT COUNT(id) FROM modified_images;")
        .fetch_one(&test_app.pool)
        .await
        .unwrap()
        .count
        .unwrap();
    let number_of_images: i64 = query!("SELECT COUNT(id) FROM images;")
        .fetch_one(&test_app.pool)
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
        let save_path = tempdir
            .path()
            .join(Path::new(&format!("{}.{}", &id.to_string(), "png")));

        image
            .image
            .save(save_path)
            .expect("Could not save test image to tempdir");
        id
    };
    LazyLock::force(&LOGGER);

    tracing::debug!("Settings up db");

    let fetcher = Fetcher::Local {
        path: tempdir.path().to_path_buf(),
    };
    let TestApp { pool, settings } = setup(fetcher).await;

    let (app, _stream) = App::new(pool.clone());

    app.run_with(settings).await.unwrap();

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

#[tokio::test]
async fn running_creates_new_run_in_db() {
    let TestApp { pool, settings } = setup(Fetcher::Picsum { images_n: 1 }).await;

    let (app, _stream) = App::new(pool.clone());

    let run_name= settings.run_name.clone();

    app.run_with(settings).await.unwrap();

    let result = sqlx::query!(
        "SELECT COUNT(id) FROM runs WHERE name = $1",
        run_name
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_some!(result.count);
}

struct TestApp {
    pub pool: PgPool,
    pub settings: Settings,
}

async fn setup(fetcher: Fetcher) -> TestApp {
    LazyLock::force(&LOGGER);

    tracing::debug!("Settings up db");
    let pool = setup_db().await;

    let hashing_methods = vec![HashingMethodType::Mean, HashingMethodType::Median];
    let modifications = vec![ModificationType::Blur, ModificationType::Contrast];

    let mut settings = Settings::default();
    let run_name = Uuid::new_v4();

    settings
        .disable_event_loop()
        .fetcher(fetcher)
        .hashing_methods(hashing_methods)
        .modifications(modifications)
        .run_name(run_name.into());

    TestApp { pool, settings }
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

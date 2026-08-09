use std::sync::LazyLock;

use phash::{App, Fetcher, HashingMethodType, ModificationType};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool, query};
use uuid::Uuid;

static LOGGER: LazyLock<()> = LazyLock::new(|| {
    tracing_subscriber::fmt::init();
});

#[tokio::test(flavor = "multi_thread")]
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
        settings.images_n = 10;
        settings.hashing_methods = hashing_methods;
        settings.modifications = modifications;
        settings.fetcher = fetcher;
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

    assert_eq!(number_of_hashes as u64, expected_number_of_hashes);
    assert_eq!(
        expected_number_of_modified_images as i64,
        number_of_modified_images
    );
    assert_eq!(images_n as i64, number_of_images);
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

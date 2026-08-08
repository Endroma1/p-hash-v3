use phash::{App, HashingMethodType, ModificationType, Settings};
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool, query};
use uuid::Uuid;

#[tokio::test]
async fn run_creates_correct_amount_of_entries_in_database() {
    let pool = setup_db().await;

    let mut settings = Settings::default();
    settings.images_n = 10;
    settings.hashing_methods = vec![HashingMethodType::Mean];
    settings.modifications = vec![ModificationType::Blur];

    let expected_number_of_results = settings.expected_number_of_results();

    let (app, _stream) = App::new(&pool);
    app.run(&settings).await.unwrap();

    let number_of_results: i64 = query!("SELECT COUNT(id) FROM hashes;")
        .fetch_one(&pool)
        .await
        .unwrap()
        .count
        .unwrap();

    assert_eq!(number_of_results as u64, expected_number_of_results);
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

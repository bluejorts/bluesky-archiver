use bluesky_archiver::archive::Archiver;
use bluesky_archiver::bluesky::Client;
use bluesky_archiver::database::Database;
use tempfile::tempdir;

async fn setup_test_archiver() -> (
    Archiver<'static>,
    tempfile::TempDir,
    tempfile::TempDir,
    Client,
) {
    let output_dir = tempdir().unwrap();
    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();

    let client = Box::leak(Box::new(Client::new()));
    let archiver = Archiver::new(db, output_dir.path().to_path_buf(), client);

    (archiver, output_dir, db_dir, client.clone())
}

#[tokio::test]
async fn test_archive_empty_posts() {
    let (archiver, _output_dir, _db_dir, _client) = setup_test_archiver().await;

    let stats = archiver.archive_posts(vec![], false).await.unwrap();
    assert_eq!(stats.downloaded, 0);
    assert_eq!(stats.skipped, 0);
    assert_eq!(stats.failed, 0);
}

#[tokio::test]
async fn test_directory_structure_creation() {
    let (archiver, output_dir, _db_dir, _client) = setup_test_archiver().await;

    let posts = vec![];
    archiver.archive_posts(posts, false).await.unwrap();

    let archive_path = output_dir.path();
    assert!(archive_path.exists());
}

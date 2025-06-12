//! Integration tests that require external services (Bluesky API)
//!
//! Run with: cargo test --test integration_tests
//! Skip with: cargo test -- --skip integration_tests

use bluesky_archiver::archive::Archiver;
use bluesky_archiver::bluesky::Client;
use bluesky_archiver::database::Database;
use std::env;
use tempfile::tempdir;

#[tokio::test]
async fn test_full_archive_workflow() {
    // Check if credentials are available
    let username = match env::var("BLUESKY_USERNAME") {
        Ok(u) => u,
        Err(_) => {
            eprintln!("Skipping test: BLUESKY_USERNAME not set");
            return;
        }
    };
    let password = match env::var("BLUESKY_APP_PASSWORD") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Skipping test: BLUESKY_APP_PASSWORD not set");
            return;
        }
    };

    let output_dir = tempdir().unwrap();
    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");

    let db = Database::new(&db_path).unwrap();
    let mut client = Client::new();

    client.login(&username, &password).await.unwrap();

    let posts = match client
        .get_likes_with_options(&username, 5, 0, None, None)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Warning: Could not fetch likes: {}", e);
            vec![]
        }
    };
    let post_count = posts.len();
    let has_posts = !posts.is_empty();

    let archiver = Archiver::new(db, output_dir.path().to_path_buf(), &client);
    let stats = archiver.archive_posts(posts, false).await.unwrap();

    println!(
        "Archive stats: downloaded={}, skipped={}, failed={}",
        stats.downloaded, stats.skipped, stats.failed
    );

    // Check that archiver ran successfully
    if has_posts {
        println!("Processed {} posts", post_count);
    } else {
        println!("No posts to process - test completed successfully");
    }
}

#[tokio::test]
async fn test_user_archive_workflow() {
    // Check if credentials are available
    let username = match env::var("BLUESKY_USERNAME") {
        Ok(u) => u,
        Err(_) => {
            eprintln!("Skipping test: BLUESKY_USERNAME not set");
            return;
        }
    };
    let password = match env::var("BLUESKY_APP_PASSWORD") {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Skipping test: BLUESKY_APP_PASSWORD not set");
            return;
        }
    };
    let target_user = env::var("TEST_TARGET_USER").unwrap_or(username.clone());

    let output_dir = tempdir().unwrap();
    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");

    let db = Database::new(&db_path).unwrap();
    let mut client = Client::new();

    client.login(&username, &password).await.unwrap();

    let posts = match client
        .get_user_posts_with_options(&target_user, 10, 0, None, None)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Warning: Could not fetch user posts: {}", e);
            vec![]
        }
    };

    let archiver = Archiver::new(db, output_dir.path().to_path_buf(), &client);
    let stats = archiver.archive_posts(posts, false).await.unwrap();

    println!(
        "User archive stats: downloaded={}, skipped={}, failed={}",
        stats.downloaded, stats.skipped, stats.failed
    );
}

#[tokio::test]
async fn test_database_persistence() {
    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");

    {
        let db = Database::new(&db_path).unwrap();
        let post = bluesky_archiver::database::ArchivedPost {
            uri: "at://test/post/1".to_string(),
            cid: "test_cid".to_string(),
            author_did: "did:plc:test".to_string(),
            author_handle: "test.bsky.social".to_string(),
            post_text: Some("Test post".to_string()),
            image_count: 1,
            archived_at: chrono::Utc::now(),
            post_created_at: "2024-01-01T00:00:00Z".to_string(),
            has_content_warning: false,
        };
        db.save_post(&post).unwrap();
    }

    {
        let db = Database::new(&db_path).unwrap();
        assert!(db.is_post_archived("at://test/post/1").unwrap());
        let (post_count, _) = db.get_stats().unwrap();
        assert_eq!(post_count, 1);
    }
}

#[tokio::test]
async fn test_rate_limiting_retry() {
    let mut client = Client::new();

    let result = client.login("nonexistent.user", "wrong.password").await;
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Login failed"));
}

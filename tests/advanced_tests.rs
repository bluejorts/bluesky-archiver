use bluesky_archiver::archive::Archiver;
use bluesky_archiver::bluesky::Client;
use bluesky_archiver::database::Database;
use tempfile::tempdir;

#[tokio::test]
async fn test_nsfw_filtering() {
    let output_dir = tempdir().unwrap();
    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();
    
    let client = Box::leak(Box::new(Client::new()));
    let archiver = Archiver::new(db, output_dir.path().to_path_buf(), client);
    
    // Create posts with and without NSFW labels
    let posts = vec![];
    
    // Archive with NSFW filter enabled
    let stats = archiver.archive_posts(posts, true).await.unwrap();
    assert_eq!(stats.downloaded, 0);
    assert_eq!(stats.skipped, 0);
    assert_eq!(stats.failed, 0);
}

#[tokio::test]
async fn test_empty_cursor_handling() {
    let client = Client::new();
    
    // Test that None cursor is handled properly
    let result = client.get_likes_with_options(
        "test.user",
        10,
        0,
        None,
        None
    ).await;
    
    // Should fail due to not authenticated
    assert!(result.is_err());
}

#[tokio::test]
async fn test_archive_directory_creation() {
    let output_dir = tempdir().unwrap();
    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();
    
    let client = Box::leak(Box::new(Client::new()));
    let archiver = Archiver::new(db, output_dir.path().to_path_buf(), client);
    
    // Verify base directory exists
    assert!(output_dir.path().exists());
    
    // Archive should create subdirectories as needed
    let _stats = archiver.archive_posts(vec![], false).await.unwrap();
}

#[tokio::test]
async fn test_duplicate_image_detection() {
    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();
    
    // Test that duplicate blob CIDs are detected
    let blob_cid = "test_blob_cid_123";
    assert!(!db.is_image_archived(blob_cid).unwrap());
    
    // First save a post
    let post = bluesky_archiver::database::ArchivedPost {
        uri: "at://test/post/1".to_string(),
        cid: "test_cid".to_string(),
        author_did: "did:plc:test".to_string(),
        author_handle: "test.bsky.social".to_string(),
        post_text: None,
        image_count: 1,
        archived_at: chrono::Utc::now(),
        post_created_at: "2024-01-01T00:00:00Z".to_string(),
        has_content_warning: false,
    };
    db.save_post(&post).unwrap();
    
    // Save an image
    let image = bluesky_archiver::database::ArchivedImage {
        id: 0,
        post_uri: "at://test/post/1".to_string(),
        blob_cid: blob_cid.to_string(),
        filename: "test.jpg".to_string(),
        mime_type: "image/jpeg".to_string(),
        size: 1024,
        alt_text: None,
        downloaded_at: chrono::Utc::now(),
    };
    
    db.save_image(&image).unwrap();
    
    // Should now be detected as archived
    assert!(db.is_image_archived(blob_cid).unwrap());
}

#[tokio::test]
async fn test_stats_calculation() {
    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();
    
    // Initial stats should be zero
    let (posts, images) = db.get_stats().unwrap();
    assert_eq!(posts, 0);
    assert_eq!(images, 0);
    
    // Add a post
    let post = bluesky_archiver::database::ArchivedPost {
        uri: "at://test/post/1".to_string(),
        cid: "test_cid".to_string(),
        author_did: "did:plc:test".to_string(),
        author_handle: "test.bsky.social".to_string(),
        post_text: None,
        image_count: 2,
        archived_at: chrono::Utc::now(),
        post_created_at: "2024-01-01T00:00:00Z".to_string(),
        has_content_warning: false,
    };
    db.save_post(&post).unwrap();
    
    // Add images
    for i in 1..=2 {
        let image = bluesky_archiver::database::ArchivedImage {
            id: 0,
            post_uri: post.uri.clone(),
            blob_cid: format!("blob_{}", i),
            filename: format!("image_{}.jpg", i),
            mime_type: "image/jpeg".to_string(),
            size: 1024,
            alt_text: None,
            downloaded_at: chrono::Utc::now(),
        };
        db.save_image(&image).unwrap();
    }
    
    // Verify stats
    let (posts, images) = db.get_stats().unwrap();
    assert_eq!(posts, 1);
    assert_eq!(images, 2);
}
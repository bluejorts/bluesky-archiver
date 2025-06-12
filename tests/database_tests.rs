use bluesky_archiver::database::{ArchivedImage, ArchivedPost, Database};
use chrono::Utc;
use tempfile::tempdir;

fn create_test_db() -> (Database, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();
    (db, temp_dir)
}

#[test]
fn test_database_creation() {
    let (db, _temp_dir) = create_test_db();
    let (posts, images) = db.get_stats().unwrap();
    assert_eq!(posts, 0);
    assert_eq!(images, 0);
}

#[test]
fn test_save_and_check_post() {
    let (db, _temp_dir) = create_test_db();

    let post = ArchivedPost {
        uri: "at://test.post/1".to_string(),
        cid: "test_cid_1".to_string(),
        author_did: "did:plc:testuser".to_string(),
        author_handle: "testuser.bsky.social".to_string(),
        post_text: Some("Test post content".to_string()),
        image_count: 1,
        archived_at: Utc::now(),
        post_created_at: "2024-01-01T00:00:00Z".to_string(),
        has_content_warning: false,
    };

    assert!(!db.is_post_archived(&post.uri).unwrap());

    db.save_post(&post).unwrap();

    assert!(db.is_post_archived(&post.uri).unwrap());

    let (post_count, _) = db.get_stats().unwrap();
    assert_eq!(post_count, 1);
}

#[test]
fn test_save_and_check_image() {
    let (db, _temp_dir) = create_test_db();

    let post = ArchivedPost {
        uri: "at://test.post/1".to_string(),
        cid: "test_cid_1".to_string(),
        author_did: "did:plc:testuser".to_string(),
        author_handle: "testuser.bsky.social".to_string(),
        post_text: None,
        image_count: 1,
        archived_at: Utc::now(),
        post_created_at: "2024-01-01T00:00:00Z".to_string(),
        has_content_warning: true,
    };

    db.save_post(&post).unwrap();

    let image = ArchivedImage {
        id: 0,
        post_uri: post.uri.clone(),
        blob_cid: "blob_cid_123".to_string(),
        filename: "test_image.jpg".to_string(),
        mime_type: "image/jpeg".to_string(),
        size: 1024,
        alt_text: Some("Test alt text".to_string()),
        downloaded_at: Utc::now(),
    };

    assert!(!db.is_image_archived(&image.blob_cid).unwrap());

    db.save_image(&image).unwrap();

    assert!(db.is_image_archived(&image.blob_cid).unwrap());

    let (_, image_count) = db.get_stats().unwrap();
    assert_eq!(image_count, 1);
}

#[test]
fn test_duplicate_post_handling() {
    let (db, _temp_dir) = create_test_db();

    let post = ArchivedPost {
        uri: "at://test.post/1".to_string(),
        cid: "test_cid_1".to_string(),
        author_did: "did:plc:testuser".to_string(),
        author_handle: "testuser.bsky.social".to_string(),
        post_text: Some("Original text".to_string()),
        image_count: 1,
        archived_at: Utc::now(),
        post_created_at: "2024-01-01T00:00:00Z".to_string(),
        has_content_warning: false,
    };

    db.save_post(&post).unwrap();

    let mut updated_post = post;
    updated_post.post_text = Some("Updated text".to_string());

    db.save_post(&updated_post).unwrap();

    let (post_count, _) = db.get_stats().unwrap();
    assert_eq!(post_count, 1);
}

#[test]
fn test_multiple_images_per_post() {
    let (db, _temp_dir) = create_test_db();

    let post = ArchivedPost {
        uri: "at://test.post/1".to_string(),
        cid: "test_cid_1".to_string(),
        author_did: "did:plc:testuser".to_string(),
        author_handle: "testuser.bsky.social".to_string(),
        post_text: None,
        image_count: 3,
        archived_at: Utc::now(),
        post_created_at: "2024-01-01T00:00:00Z".to_string(),
        has_content_warning: false,
    };

    db.save_post(&post).unwrap();

    for i in 1..=3 {
        let image = ArchivedImage {
            id: 0,
            post_uri: post.uri.clone(),
            blob_cid: format!("blob_cid_{}", i),
            filename: format!("image_{}.jpg", i),
            mime_type: "image/jpeg".to_string(),
            size: 1024 * i as i64,
            alt_text: Some(format!("Image {} alt text", i)),
            downloaded_at: Utc::now(),
        };
        db.save_image(&image).unwrap();
    }

    let (post_count, image_count) = db.get_stats().unwrap();
    assert_eq!(post_count, 1);
    assert_eq!(image_count, 3);
}

#[test]
fn test_special_characters_in_text() {
    let (db, _temp_dir) = create_test_db();

    let post = ArchivedPost {
        uri: "at://test.post/1".to_string(),
        cid: "test_cid_1".to_string(),
        author_did: "did:plc:testuser".to_string(),
        author_handle: "testuser.bsky.social".to_string(),
        post_text: Some("Text with 'quotes' and \"double quotes\" and emoji 🎉".to_string()),
        image_count: 0,
        archived_at: Utc::now(),
        post_created_at: "2024-01-01T00:00:00Z".to_string(),
        has_content_warning: false,
    };

    db.save_post(&post).unwrap();
    assert!(db.is_post_archived(&post.uri).unwrap());
}

use bluesky_archiver::bluesky::{Client, Embed, Post};
use bluesky_archiver::archive::Archiver;
use bluesky_archiver::database::Database;
use serde_json::json;
use tempfile::TempDir;

#[tokio::test]
async fn test_archive_with_new_blob_format() {
    // Create a test post with the new blob format that was causing issues
    let post_json = json!({
        "uri": "at://did:plc:test/app.bsky.feed.post/3kqy6gvh2gk2x",
        "cid": "bafyreihgb3q5lqkqpfkrw5qaussicc7fnymu3fqvvmq4gpjqfgqsqut2h4",
        "author": {
            "did": "did:plc:testuser123",
            "handle": "testuser.bsky.social",
            "displayName": "Test User"
        },
        "record": {
            "$type": "app.bsky.feed.post",
            "text": "Test post with image",
            "createdAt": "2024-01-01T12:00:00.000Z",
            "embed": {
                "$type": "app.bsky.embed.images",
                "images": [{
                    "alt": "Test image alt text",
                    "fullsize": "https://cdn.bsky.app/img/feed_fullsize/plain/did:plc:test/bafkreihjr5hfxbqovqiw4dci5rqhm3ebvn6pbylmupfxj5vjvawmkjeulm@jpeg",
                    "thumb": "https://cdn.bsky.app/img/feed_thumbnail/plain/did:plc:test/bafkreihjr5hfxbqovqiw4dci5rqhm3ebvn6pbylmupfxj5vjvawmkjeulm@jpeg",
                    "aspectRatio": {
                        "width": 1920,
                        "height": 1080
                    },
                    "image": {
                        "$type": "blob",
                        "ref": {
                            "$link": "bafkreihjr5hfxbqovqiw4dci5rqhm3ebvn6pbylmupfxj5vjvawmkjeulm"
                        },
                        "mimeType": "image/jpeg",
                        "size": 234567
                    }
                }]
            }
        },
        "indexedAt": "2024-01-01T12:00:00.000Z",
        "labels": []
    });

    let post: Post = serde_json::from_value(post_json).unwrap();
    
    // Verify the post parses correctly
    assert_eq!(post.uri, "at://did:plc:test/app.bsky.feed.post/3kqy6gvh2gk2x");
    
    // Check that we can extract the embed
    if let Some(embed_value) = post.record.get("embed") {
        let embed: Embed = serde_json::from_value(embed_value.clone()).unwrap();
        match embed {
            Embed::Images { images, .. } => {
                assert_eq!(images.len(), 1);
                assert_eq!(images[0].image.ref_.link, "bafkreihjr5hfxbqovqiw4dci5rqhm3ebvn6pbylmupfxj5vjvawmkjeulm");
                assert_eq!(images[0].image.mime_type, "image/jpeg");
                assert_eq!(images[0].image.size, 234567);
            }
            _ => panic!("Expected Images embed, got {:?}", embed),
        }
    } else {
        panic!("No embed found in post");
    }

    // Test that archiver would process this post correctly
    // We can't call extract_images directly since it's private, but we can verify
    // that the post structure is correct for the archiver to handle
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();
    let output_dir = temp_dir.path().to_path_buf();
    let client = Client::new();
    
    let _archiver = Archiver::new(db, output_dir, &client);
    
    // The important thing is that the post and embed parse correctly
    // which we've already verified above
}

#[test]
fn test_real_world_api_response_parsing() {
    // Test with actual API response format from the error logs
    let api_response = json!({
        "uri": "at://did:plc:k6f35t4xmscmipze4n5x4kaw/app.bsky.feed.post/3kqgfvrvngk2x",
        "cid": "bafyreifdbrvhcvkumhmskegkg5goh7gqaahkjupiyl7xiccoixvgwgqbni",
        "author": {
            "did": "did:plc:k6f35t4xmscmipze4n5x4kaw",
            "handle": "shiratamarie.bsky.social"
        },
        "record": {
            "$type": "app.bsky.feed.post",
            "createdAt": "2024-01-01T00:00:00.000Z",
            "embed": {
                "$type": "app.bsky.embed.images",
                "images": [{
                    "alt": "",
                    "aspectRatio": {
                        "height": 1638,
                        "width": 2048
                    },
                    "image": {
                        "$type": "blob",
                        "ref": {
                            "$link": "bafkreifpefmovxq4af25wuqjflzb4y2kycgh5l3ufvmdvlghcxfgdayeee"
                        },
                        "mimeType": "image/jpeg",
                        "size": 992397
                    }
                }]
            },
            "langs": ["ja"],
            "text": "Test text"
        },
        "replyCount": 0,
        "repostCount": 4,
        "likeCount": 70,
        "indexedAt": "2024-01-01T00:00:00.000Z",
        "labels": []
    });

    // Test parsing the full post
    let post: Post = serde_json::from_value(api_response).unwrap();
    
    // The important thing is that the archiver can extract images from this format
    // Let's test the same way the archiver does it
    if let Some(embed_value) = post.record.get("embed") {
        // Check that the embed has the right type
        let embed_type = embed_value.get("$type").and_then(|v| v.as_str()).unwrap();
        assert_eq!(embed_type, "app.bsky.embed.images");
        
        // The archiver would parse it like this, which should work
        let result = serde_json::from_value::<Embed>(embed_value.clone());
        assert!(result.is_ok(), "Failed to parse embed: {:?}", result.err());
    } else {
        panic!("No embed found in post");
    }
}

#[test]
fn test_multiple_images_parsing() {
    // Test post with multiple images
    let post_json = json!({
        "uri": "at://did:plc:test/app.bsky.feed.post/multi",
        "cid": "bafyreimulti",
        "author": {
            "did": "did:plc:testuser",
            "handle": "test.bsky.social"
        },
        "record": {
            "$type": "app.bsky.feed.post",
            "text": "Multiple images",
            "createdAt": "2024-01-01T12:00:00.000Z",
            "embed": {
                "$type": "app.bsky.embed.images",
                "images": [
                    {
                        "alt": "Image 1",
                        "fullsize": "https://example.com/1.jpg",
                        "thumb": "https://example.com/1_thumb.jpg",
                        "image": {
                            "$type": "blob",
                            "ref": {
                                "$link": "bafkreimage1"
                            },
                            "mimeType": "image/jpeg",
                            "size": 100000
                        }
                    },
                    {
                        "alt": "Image 2",
                        "fullsize": "https://example.com/2.jpg",
                        "thumb": "https://example.com/2_thumb.jpg",
                        "image": {
                            "$type": "blob",
                            "ref": {
                                "$link": "bafkreimage2"
                            },
                            "mimeType": "image/png",
                            "size": 200000
                        }
                    }
                ]
            }
        },
        "indexedAt": "2024-01-01T12:00:00.000Z",
        "labels": []
    });

    let post: Post = serde_json::from_value(post_json).unwrap();
    let embed_value = post.record.get("embed").unwrap();
    let embed: Embed = serde_json::from_value(embed_value.clone()).unwrap();
    
    match embed {
        Embed::Images { images, .. } => {
            assert_eq!(images.len(), 2);
            assert_eq!(images[0].image.ref_.link, "bafkreimage1");
            assert_eq!(images[1].image.ref_.link, "bafkreimage2");
            assert_eq!(images[0].alt.as_deref(), Some("Image 1"));
            assert_eq!(images[1].alt.as_deref(), Some("Image 2"));
        }
        _ => panic!("Expected Images embed"),
    }
}
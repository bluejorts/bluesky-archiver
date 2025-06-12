use bluesky_archiver::bluesky::{Client, Embed, Post};
use serde_json::json;

#[tokio::test]
async fn test_client_creation() {
    let _client = Client::new();
    // Client creates successfully
}

#[tokio::test]
async fn test_get_likes_not_authenticated() {
    let client = Client::new();
    let result = client.get_likes_with_options("test.bsky.social", 10, 0, None, None).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Not authenticated"));
}

#[tokio::test]
async fn test_post_has_nsfw_labels() {
    let post_json = json!({
        "uri": "at://test/post/1",
        "cid": "cid1",
        "author": {
            "did": "did:plc:test",
            "handle": "test.handle"
        },
        "record": {
            "$type": "app.bsky.feed.post",
            "text": "Test",
            "createdAt": "2024-01-01T00:00:00Z"
        },
        "indexedAt": "2024-01-01T00:00:00Z",
        "labels": [
            {
                "src": "did:plc:moderator",
                "uri": "at://test/post/1",
                "val": "porn",
                "cts": "2024-01-01T00:00:00Z"
            }
        ]
    });

    let post: Post = serde_json::from_value(post_json).unwrap();
    assert!(post.has_nsfw_labels());

    let safe_post_json = json!({
        "uri": "at://test/post/2",
        "cid": "cid2",
        "author": {
            "did": "did:plc:test",
            "handle": "test.handle"
        },
        "record": {
            "$type": "app.bsky.feed.post",
            "text": "Safe post",
            "createdAt": "2024-01-01T00:00:00Z"
        },
        "indexedAt": "2024-01-01T00:00:00Z",
        "labels": []
    });

    let safe_post: Post = serde_json::from_value(safe_post_json).unwrap();
    assert!(!safe_post.has_nsfw_labels());
}

#[tokio::test]
async fn test_embed_parsing() {
    let image_embed_json = json!({
        "$type": "app.bsky.embed.images",
        "images": [{
            "alt": "Test image",
            "fullsize": "https://example.com/full.jpg",
            "thumb": "https://example.com/thumb.jpg",
            "aspectRatio": {
                "width": 1920,
                "height": 1080
            },
            "image": {
                "cid": "bafyimage123",
                "mimeType": "image/jpeg"
            }
        }]
    });

    let embed: Embed = serde_json::from_value(image_embed_json).unwrap();
    match embed {
        Embed::Images { images, .. } => {
            assert_eq!(images.len(), 1);
            assert_eq!(images[0].alt.as_deref(), Some("Test image"));
        }
        _ => panic!("Expected Images embed"),
    }

    let external_embed_json = json!({
        "$type": "app.bsky.embed.external",
        "external": {
            "uri": "https://example.com",
            "title": "Example",
            "description": "Example site"
        }
    });

    let external_embed: Embed = serde_json::from_value(external_embed_json).unwrap();
    match external_embed {
        Embed::External { external, .. } => {
            assert_eq!(external.uri, "https://example.com");
            assert_eq!(external.title, "Example");
        }
        _ => panic!("Expected External embed"),
    }
}

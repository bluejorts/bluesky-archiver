use anyhow::{anyhow, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde_json::json;
use std::time::Instant;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

const API_BASE: &str = "https://bsky.social/xrpc";

type CursorCallback = Box<dyn Fn(&str) + Send>;

#[derive(Debug, Clone)]
pub struct Client {
    http: HttpClient,
    session: Option<Session>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct Session {
    did: String,
    #[serde(rename = "accessJwt")]
    access_jwt: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    did: String,
    #[serde(rename = "accessJwt")]
    access_jwt: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Post {
    pub uri: String,
    pub cid: String,
    pub author: Author,
    pub record: serde_json::Value,
    #[serde(rename = "indexedAt")]
    pub indexed_at: String,
    pub labels: Option<Vec<Label>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Label {
    pub src: String,
    pub uri: String,
    pub val: String,
    #[serde(rename = "cts")]
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Author {
    pub did: String,
    pub handle: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Record {
    #[serde(rename = "$type")]
    pub record_type: String,
    pub text: Option<String>,
    pub embed: Option<Embed>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
pub enum Embed {
    Images {
        #[serde(rename = "$type")]
        embed_type: String,
        images: Vec<Image>,
    },
    External {
        #[serde(rename = "$type")]
        embed_type: String,
        external: External,
    },
    Other(serde_json::Value),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Image {
    pub alt: Option<String>,
    pub fullsize: String,
    pub thumb: String,
    #[serde(rename = "aspectRatio")]
    pub aspect_ratio: Option<AspectRatio>,
    pub image: View,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct AspectRatio {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct View {
    #[serde(rename = "$type")]
    pub type_: String,
    #[serde(rename = "ref")]
    pub ref_: BlobRef,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BlobRef {
    #[serde(rename = "$link")]
    pub link: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct External {
    pub uri: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
struct GetLikesResponse {
    pub feed: Vec<FeedItem>,
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FeedItem {
    pub post: Post,
}

#[derive(Debug, Deserialize)]
struct GetAuthorFeedResponse {
    pub feed: Vec<AuthorFeedItem>,
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthorFeedItem {
    pub post: Post,
    pub reason: Option<serde_json::Value>, // Used to identify reposts
}

impl Post {
    pub fn has_nsfw_labels(&self) -> bool {
        if let Some(labels) = &self.labels {
            labels.iter().any(|label| {
                matches!(
                    label.val.as_str(),
                    "porn"
                        | "sexual"
                        | "nudity"
                        | "graphic-media"
                        | "self-harm"
                        | "sensitive"
                        | "content-warning"
                )
            })
        } else {
            false
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
            http: HttpClient::new(),
            session: None,
        }
    }
}

impl Client {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn login(&mut self, identifier: &str, password: &str) -> Result<()> {
        let url = format!("{}/com.atproto.server.createSession", API_BASE);

        let body = json!({
            "identifier": identifier,
            "password": password
        });

        let response = self.http.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Login failed: {}", error_text));
        }

        let login_response: LoginResponse = response.json().await?;

        self.session = Some(Session {
            did: login_response.did.clone(),
            access_jwt: login_response.access_jwt,
        });

        info!("Successfully logged in as DID: {}", login_response.did);
        Ok(())
    }

    pub async fn get_likes_with_options(
        &self,
        actor: &str,
        limit: usize,
        delay_ms: u64,
        start_cursor: Option<String>,
        cursor_callback: Option<CursorCallback>,
    ) -> Result<Vec<Post>> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("Not authenticated"))?;

        let mut all_posts = Vec::new();
        let mut cursor: Option<String> = start_cursor;
        // Use larger page size for better performance, API supports up to 100
        let page_size = if limit == 0 { 100 } else { 100.min(limit) };

        if cursor.is_some() {
            info!("Resuming from saved cursor position");
        }

        // Create progress bar
        let pb = if limit == 0 {
            ProgressBar::new_spinner()
        } else {
            ProgressBar::new(limit as u64)
        };

        pb.set_style(
            if limit == 0 {
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {pos} posts fetched ({per_sec}) {msg}")?
            } else {
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} posts ({per_sec}) {msg}")?
                    .progress_chars("=>-")
            }
        );

        if limit == 0 {
            pb.set_message("Fetching all liked posts...");
        }

        let mut retry_count = 0;
        let max_retries = 5;

        loop {
            let url = format!("{}/app.bsky.feed.getActorLikes", API_BASE);

            let mut params = vec![
                ("actor", actor.to_string()),
                ("limit", page_size.to_string()),
            ];

            if let Some(ref c) = cursor {
                params.push(("cursor", c.clone()));
            }

            // Add delay if specified
            if delay_ms > 0 && !all_posts.is_empty() {
                sleep(Duration::from_millis(delay_ms)).await;
            }

            let _request_start = Instant::now();
            let response = self
                .http
                .get(&url)
                .bearer_auth(&session.access_jwt)
                .query(&params)
                .send()
                .await?;

            let status = response.status();

            // Handle rate limiting
            if status.as_u16() == 429 {
                retry_count += 1;
                if retry_count > max_retries {
                    pb.finish_and_clear();
                    return Err(anyhow!(
                        "Rate limited after {} retries. Try again later or use --delay flag",
                        max_retries
                    ));
                }

                let wait_time = 2u64.pow(retry_count) * 1000; // Exponential backoff in ms
                pb.set_message(format!(
                    "Rate limited! Waiting {}s before retry {}/{}...",
                    wait_time / 1000,
                    retry_count,
                    max_retries
                ));
                sleep(Duration::from_millis(wait_time)).await;
                continue;
            }

            if !status.is_success() {
                let error_text = response.text().await?;
                pb.finish_and_clear();
                return Err(anyhow!(
                    "Failed to fetch likes: {} - {}",
                    status,
                    error_text
                ));
            }

            // Reset retry count on success
            retry_count = 0;

            let response_text = response.text().await?;
            let likes_response: GetLikesResponse = match serde_json::from_str(&response_text) {
                Ok(resp) => resp,
                Err(e) => {
                    // Log the error and response for debugging
                    warn!("Failed to parse likes response: {}", e);
                    warn!("Response text: {}", response_text);
                    pb.finish_and_clear();
                    return Err(anyhow!("Failed to parse likes response: {}", e));
                }
            };

            let new_posts = likes_response.feed.len();

            // Check if we got any posts
            if new_posts == 0 {
                info!("No more posts returned, reached end of likes");
                break;
            }

            for item in likes_response.feed {
                all_posts.push(item.post);

                if limit > 0 {
                    pb.inc(1);
                    if all_posts.len() >= limit {
                        pb.finish_with_message("Fetching complete");
                        return Ok(all_posts);
                    }
                }
            }

            if limit == 0 {
                pb.set_message(format!("Fetched {} posts...", all_posts.len()));
                pb.inc(new_posts as u64);
            }

            cursor = likes_response.cursor;

            if cursor.is_none() {
                info!("No cursor returned, reached end of likes");
                break;
            }

            // Save cursor position if callback provided
            if let (Some(ref c), Some(ref callback)) = (&cursor, &cursor_callback) {
                callback(c);
            }
        }

        pb.finish_with_message(format!("Fetched all {} posts", all_posts.len()));
        info!("Total posts fetched: {}", all_posts.len());
        Ok(all_posts)
    }

    pub async fn get_user_posts_with_options(
        &self,
        actor: &str,
        limit: usize,
        delay_ms: u64,
        start_cursor: Option<String>,
        cursor_callback: Option<CursorCallback>,
    ) -> Result<Vec<Post>> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("Not authenticated"))?;

        let mut all_posts = Vec::new();
        let mut cursor: Option<String> = start_cursor;
        let page_size = if limit == 0 { 100 } else { 100.min(limit) };

        if cursor.is_some() {
            info!("Resuming from saved cursor position");
        }

        // Create progress bar
        let pb = if limit == 0 {
            ProgressBar::new_spinner()
        } else {
            ProgressBar::new(limit as u64)
        };

        pb.set_style(
            if limit == 0 {
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {pos} posts fetched ({per_sec}) {msg}")?
            } else {
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} posts ({per_sec}) {msg}")?
                    .progress_chars("=>-")
            }
        );

        if limit == 0 {
            pb.set_message("Fetching all user posts...");
        }

        let mut retry_count = 0;
        let max_retries = 5;

        loop {
            let url = format!("{}/app.bsky.feed.getAuthorFeed", API_BASE);

            let mut params = vec![
                ("actor", actor.to_string()),
                ("limit", page_size.to_string()),
                ("filter", "posts_with_media".to_string()), // Only get posts with media
            ];

            if let Some(ref c) = cursor {
                params.push(("cursor", c.clone()));
            }

            // Add delay if specified
            if delay_ms > 0 && !all_posts.is_empty() {
                sleep(Duration::from_millis(delay_ms)).await;
            }

            let response = self
                .http
                .get(&url)
                .bearer_auth(&session.access_jwt)
                .query(&params)
                .send()
                .await?;

            let status = response.status();

            // Handle rate limiting
            if status.as_u16() == 429 {
                retry_count += 1;
                if retry_count > max_retries {
                    pb.finish_and_clear();
                    return Err(anyhow!(
                        "Rate limited after {} retries. Try again later or use --delay flag",
                        max_retries
                    ));
                }

                let wait_time = 2u64.pow(retry_count) * 1000; // Exponential backoff in ms
                pb.set_message(format!(
                    "Rate limited! Waiting {}s before retry {}/{}...",
                    wait_time / 1000,
                    retry_count,
                    max_retries
                ));
                sleep(Duration::from_millis(wait_time)).await;
                continue;
            }

            if !status.is_success() {
                let error_text = response.text().await?;
                pb.finish_and_clear();
                return Err(anyhow!(
                    "Failed to fetch user posts: {} - {}",
                    status,
                    error_text
                ));
            }

            // Reset retry count on success
            retry_count = 0;

            let response_text = response.text().await?;
            let feed_response: GetAuthorFeedResponse = match serde_json::from_str(&response_text) {
                Ok(resp) => resp,
                Err(e) => {
                    warn!("Failed to parse feed response: {}", e);
                    warn!("Response text: {}", response_text);
                    pb.finish_and_clear();
                    return Err(anyhow!("Failed to parse feed response: {}", e));
                }
            };

            let mut new_posts_count = 0;
            let feed_items = feed_response.feed;
            let feed_empty = feed_items.is_empty();

            // Filter out reposts and quote posts, only keep original posts with images
            for item in feed_items {
                // Skip if it's a repost (has reason field)
                if item.reason.is_some() {
                    continue;
                }

                let post = item.post;

                // Skip quote posts (check if embed type is record)
                if let Some(embed_value) = post.record.get("embed") {
                    if let Some(embed_type) = embed_value.get("$type").and_then(|v| v.as_str()) {
                        if embed_type.contains("record") {
                            continue; // Skip quote posts
                        }
                    }
                }

                // Only include posts with image embeds
                if let Some(embed_value) = post.record.get("embed") {
                    if let Ok(Embed::Images { .. }) =
                        serde_json::from_value::<Embed>(embed_value.clone())
                    {
                        all_posts.push(post);
                        new_posts_count += 1;

                        if limit > 0 {
                            pb.inc(1);
                            if all_posts.len() >= limit {
                                pb.finish_with_message("Fetching complete");
                                return Ok(all_posts);
                            }
                        }
                    }
                }
            }

            if limit == 0 {
                pb.set_message(format!("Fetched {} image posts...", all_posts.len()));
                pb.inc(new_posts_count);
            }

            // Check if we got any posts at all
            if feed_empty {
                info!("No more posts returned, reached end of feed");
                break;
            }

            cursor = feed_response.cursor;

            if cursor.is_none() {
                info!("No cursor returned, reached end of feed");
                break;
            }

            // Save cursor position if callback provided
            if let (Some(ref c), Some(ref callback)) = (&cursor, &cursor_callback) {
                callback(c);
            }
        }

        pb.finish_with_message(format!("Fetched {} image posts", all_posts.len()));
        Ok(all_posts)
    }

    pub fn get_image_url(&self, did: &str, cid: &str) -> String {
        format!(
            "https://bsky.social/xrpc/com.atproto.sync.getBlob?did={}&cid={}",
            did, cid
        )
    }

    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        let session = self
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("Not authenticated"))?;

        let response = self
            .http
            .get(url)
            .bearer_auth(&session.access_jwt)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to download image: {}", response.status()));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

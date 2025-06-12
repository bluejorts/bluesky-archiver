use anyhow::Result;
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

use crate::bluesky::{Embed, Image, Post};
use crate::database::{ArchivedImage, ArchivedPost, Database};

pub struct Archiver<'a> {
    db: Database,
    output_dir: PathBuf,
    client: &'a crate::bluesky::Client,
}

#[derive(Debug)]
pub struct ArchiveStats {
    pub downloaded: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl<'a> Archiver<'a> {
    pub fn new(db: Database, output_dir: PathBuf, client: &'a crate::bluesky::Client) -> Self {
        Self {
            db,
            output_dir,
            client,
        }
    }

    pub async fn archive_posts(&self, posts: Vec<Post>, nsfw_only: bool) -> Result<ArchiveStats> {
        let mut stats = ArchiveStats {
            downloaded: 0,
            skipped: 0,
            failed: 0,
        };

        // Filter posts based on nsfw_only flag
        let posts_to_process: Vec<_> = posts
            .into_iter()
            .filter(|post| {
                let is_nsfw = post.has_nsfw_labels();
                !nsfw_only || is_nsfw
            })
            .collect();

        if posts_to_process.is_empty() {
            info!("No posts to process after filtering");
            return Ok(stats);
        }

        // Count total images to process
        let total_images: usize = posts_to_process
            .iter()
            .map(|p| self.extract_images(p).len())
            .sum();

        info!(
            "Processing {} posts with {} total images",
            posts_to_process.len(),
            total_images
        );

        // Create progress bar
        let pb = ProgressBar::new(total_images as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} images ({per_sec}) | {msg}")?
                .progress_chars("=>-")
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(100));

        // Process posts sequentially (database isn't thread-safe)
        for post in posts_to_process.iter() {
            let is_nsfw = post.has_nsfw_labels();
            pb.set_message(format!("Processing @{}", post.author.handle));

            match self.archive_post(post, is_nsfw).await {
                Ok((downloaded, skipped)) => {
                    stats.downloaded += downloaded;
                    stats.skipped += skipped;
                    pb.inc((downloaded + skipped) as u64);
                }
                Err(e) => {
                    warn!("Failed to archive post {}: {}", post.uri, e);
                    stats.failed += 1;
                }
            }
        }

        pb.finish_with_message(format!(
            "Complete! Downloaded: {}, Skipped: {}, Failed: {}",
            stats.downloaded, stats.skipped, stats.failed
        ));

        Ok(stats)
    }

    async fn archive_post(&self, post: &Post, is_nsfw: bool) -> Result<(usize, usize)> {
        // Check if we've already processed this post
        if self.db.is_post_archived(&post.uri)? {
            debug!(
                "Post {} already archived, checking for new images",
                post.uri
            );
        }

        let images = self.extract_images(post);
        if images.is_empty() {
            debug!("No images found in post {}", post.uri);
            return Ok((0, 0));
        }

        // Save post metadata
        let archived_post = ArchivedPost {
            uri: post.uri.clone(),
            cid: post.cid.clone(),
            author_did: post.author.did.clone(),
            author_handle: post.author.handle.clone(),
            post_text: post
                .record
                .get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            image_count: images.len() as i32,
            archived_at: Utc::now(),
            post_created_at: post
                .record
                .get("createdAt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            has_content_warning: is_nsfw,
        };
        self.db.save_post(&archived_post)?;

        // Create author directory, with NSFW subdirectory if needed
        let base_dir = if is_nsfw {
            self.output_dir.join("nsfw")
        } else {
            self.output_dir.clone()
        };
        let author_dir = base_dir.join(&post.author.handle);
        fs::create_dir_all(&author_dir).await?;

        let mut downloaded = 0;
        let mut skipped = 0;

        // Download each image
        for (idx, image) in images.iter().enumerate() {
            let blob_cid = &image.image.ref_.link;

            // Check if already downloaded
            if self.db.is_image_archived(blob_cid)? {
                debug!("Image {} already downloaded", blob_cid);
                skipped += 1;
                continue;
            }

            // Generate filename
            let extension = match image.image.mime_type.as_str() {
                "image/jpeg" => "jpg",
                "image/png" => "png",
                "image/gif" => "gif",
                "image/webp" => "webp",
                _ => "bin",
            };

            let timestamp = post
                .record
                .get("createdAt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .replace(":", "-")
                .replace(".", "-");
            let filename = format!(
                "{}_{}_{}_{}.{}",
                post.author.handle,
                timestamp,
                &post.cid[..8],
                idx,
                extension
            );
            let file_path = author_dir.join(&filename);

            // Download image
            match self
                .download_image(&post.author.did, blob_cid, &file_path)
                .await
            {
                Ok(size) => {
                    // Save to database
                    let archived_image = ArchivedImage {
                        id: 0, // auto-increment
                        post_uri: post.uri.clone(),
                        blob_cid: blob_cid.clone(),
                        filename: filename.clone(),
                        mime_type: image.image.mime_type.clone(),
                        size: size as i64,
                        alt_text: image.alt.clone().filter(|s| !s.is_empty()),
                        downloaded_at: Utc::now(),
                    };
                    self.db.save_image(&archived_image)?;

                    info!("Downloaded: {}", filename);
                    downloaded += 1;
                }
                Err(e) => {
                    warn!("Failed to download image {}: {}", blob_cid, e);
                }
            }
        }

        Ok((downloaded, skipped))
    }

    fn extract_images(&self, post: &Post) -> Vec<Image> {
        if let Some(embed_value) = post.record.get("embed") {
            debug!("Found embed in post: {:?}", embed_value);
            if let Ok(Embed::Images {
                images: img_list, ..
            }) = serde_json::from_value::<Embed>(embed_value.clone())
            {
                debug!("Successfully parsed {} images from embed", img_list.len());
                return img_list;
            } else {
                debug!("Failed to parse embed as Images type");
            }
        } else {
            debug!("No embed found in post record");
        }

        Vec::new()
    }

    async fn download_image(&self, did: &str, blob_cid: &str, path: &PathBuf) -> Result<u64> {
        let url = self.client.get_image_url(did, blob_cid);
        let bytes = self.client.download_image(&url).await?;
        let size = bytes.len() as u64;

        fs::write(path, bytes).await?;

        Ok(size)
    }
}

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug)]
pub struct Database {
    conn: Connection,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchivedPost {
    pub uri: String,
    pub cid: String,
    pub author_did: String,
    pub author_handle: String,
    pub post_text: Option<String>,
    pub image_count: i32,
    pub archived_at: DateTime<Utc>,
    pub post_created_at: String,
    pub has_content_warning: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchivedImage {
    pub id: i64,
    pub post_uri: String,
    pub blob_cid: String,
    pub filename: String,
    pub mime_type: String,
    pub size: i64,
    pub alt_text: Option<String>,
    pub downloaded_at: DateTime<Utc>,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        let db = Self { conn };
        db.create_tables()?;

        Ok(db)
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS archived_posts (
                uri TEXT PRIMARY KEY,
                cid TEXT NOT NULL,
                author_did TEXT NOT NULL,
                author_handle TEXT NOT NULL,
                post_text TEXT,
                image_count INTEGER NOT NULL,
                archived_at TEXT NOT NULL,
                post_created_at TEXT NOT NULL,
                has_content_warning INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS archived_images (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                post_uri TEXT NOT NULL,
                blob_cid TEXT NOT NULL UNIQUE,
                filename TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                size INTEGER NOT NULL,
                alt_text TEXT,
                downloaded_at TEXT NOT NULL,
                FOREIGN KEY (post_uri) REFERENCES archived_posts(uri)
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_post_uri ON archived_images(post_uri)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_blob_cid ON archived_images(blob_cid)",
            [],
        )?;

        Ok(())
    }

    pub fn is_post_archived(&self, uri: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM archived_posts WHERE uri = ?1",
            params![uri],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    pub fn is_image_archived(&self, blob_cid: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM archived_images WHERE blob_cid = ?1",
            params![blob_cid],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    pub fn save_post(&self, post: &ArchivedPost) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO archived_posts 
             (uri, cid, author_did, author_handle, post_text, image_count, archived_at, post_created_at, has_content_warning)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                post.uri,
                post.cid,
                post.author_did,
                post.author_handle,
                post.post_text,
                post.image_count,
                post.archived_at.to_rfc3339(),
                post.post_created_at,
                post.has_content_warning as i32,
            ],
        )?;

        Ok(())
    }

    pub fn save_image(&self, image: &ArchivedImage) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO archived_images 
             (post_uri, blob_cid, filename, mime_type, size, alt_text, downloaded_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                image.post_uri,
                image.blob_cid,
                image.filename,
                image.mime_type,
                image.size,
                image.alt_text,
                image.downloaded_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_stats(&self) -> Result<(i64, i64)> {
        let post_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM archived_posts", [], |row| row.get(0))?;

        let image_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM archived_images", [], |row| row.get(0))?;

        Ok((post_count, image_count))
    }
}

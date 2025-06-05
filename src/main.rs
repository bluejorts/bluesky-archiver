use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::{info, warn};
use tracing_subscriber;

mod bluesky;
mod archive;
mod database;

#[derive(Parser, Debug)]
#[command(name = "bluesky-archiver")]
#[command(about = "Archive liked image posts from Bluesky", long_about = None)]
struct Args {
    /// Bluesky username (without @)
    #[arg(short, long)]
    username: String,

    /// Directory to save archived images
    #[arg(short, long, default_value = "./archive")]
    output: PathBuf,

    /// Bluesky app password (not your main password!)
    #[arg(short, long, env = "BLUESKY_APP_PASSWORD")]
    password: String,

    /// Maximum number of posts to fetch per run (0 = unlimited)
    #[arg(short, long, default_value = "100")]
    limit: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Only archive posts with NSFW/content warning labels
    #[arg(long)]
    nsfw_only: bool,

    /// Delay between API requests in milliseconds (helps avoid rate limits)
    #[arg(short, long, default_value = "0")]
    delay: u64,

    /// Resume from last saved position (useful for large archives)
    #[arg(long)]
    resume: bool,

    /// Archive all image posts from a specific user (without @)
    #[arg(long)]
    archive_user: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Set up logging
    let log_level = if args.verbose { "debug" } else { "info" };
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(log_level))
        .init();

    info!("Starting Bluesky archiver for user: {}", args.username);

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&args.output)?;

    // Initialize database
    let db_path = args.output.join("archive.db");
    let db = database::Database::new(&db_path)?;

    // Create Bluesky client and authenticate
    let mut client = bluesky::Client::new();
    client.login(&args.username, &args.password).await?;

    // Check if we're archiving a specific user's posts or liked posts
    if let Some(target_user) = args.archive_user {
        info!("Archiving all image posts from user: {}", target_user);
        
        // Fetch user's posts
        let cursor_file = args.output.join(format!(".cursor_{}", target_user));
        let start_cursor = if args.resume && cursor_file.exists() {
            match std::fs::read_to_string(&cursor_file) {
                Ok(cursor) => {
                    info!("Resuming from saved cursor");
                    Some(cursor.trim().to_string())
                }
                Err(e) => {
                    warn!("Failed to read cursor file: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        let cursor_file_clone = cursor_file.clone();
        let posts = client.get_user_posts_with_options(
            &target_user,
            args.limit,
            args.delay,
            start_cursor,
            Some(Box::new(move |cursor| {
                if let Err(e) = std::fs::write(&cursor_file_clone, cursor) {
                    warn!("Failed to save cursor: {}", e);
                }
            }))
        ).await?;
        
        // Clear cursor on successful completion
        if cursor_file.exists() {
            let _ = std::fs::remove_file(&cursor_file);
        }
        
        // Archive images from user's posts
        let archiver = archive::Archiver::new(db, args.output, &client);
        let stats = archiver.archive_posts(posts, args.nsfw_only).await?;
        
        info!(
            "Archive complete. Downloaded: {}, Skipped: {}, Failed: {}",
            stats.downloaded, stats.skipped, stats.failed
        );
    } else {
        // Original behavior: fetch liked posts
        let cursor_file = args.output.join(".cursor");
        let start_cursor = if args.resume && cursor_file.exists() {
            match std::fs::read_to_string(&cursor_file) {
                Ok(cursor) => {
                    info!("Resuming from saved cursor");
                    Some(cursor.trim().to_string())
                }
                Err(e) => {
                    warn!("Failed to read cursor file: {}", e);
                    None
                }
            }
        } else {
            None
        };
        
        let cursor_file_clone = cursor_file.clone();
        let likes = client.get_likes_with_options(
            &args.username, 
            args.limit, 
            args.delay,
            start_cursor,
            Some(Box::new(move |cursor| {
                if let Err(e) = std::fs::write(&cursor_file_clone, cursor) {
                    warn!("Failed to save cursor: {}", e);
                }
            }))
        ).await?;
        
        // Clear cursor on successful completion
        if cursor_file.exists() {
            let _ = std::fs::remove_file(&cursor_file);
        }

        // Archive images from liked posts
        let archiver = archive::Archiver::new(db, args.output, &client);
        let stats = archiver.archive_posts(likes, args.nsfw_only).await?;
        
        info!(
            "Archive complete. Downloaded: {}, Skipped: {}, Failed: {}",
            stats.downloaded, stats.skipped, stats.failed
        );
    }

    Ok(())
}
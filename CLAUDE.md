# CLAUDE.md - Project Context for AI Assistants

## Project Overview
Bluesky Archiver - A Rust application for archiving Bluesky social media posts and data. It can archive liked posts or all image posts from specific users.

## Key Commands

### Build
```bash
cargo build
cargo build --release
```

### Run
```bash
cargo run
cargo run --release
```

### Test
```bash
cargo test
```

### Lint & Format
```bash
cargo fmt
cargo clippy
```

### Docker
```bash
docker-compose up
docker build -t bluesky-archiver .
# Push to Docker Hub (requires login)
docker tag bluesky-archiver:latest polymetric/bluesky-archiver:latest
docker push polymetric/bluesky-archiver:latest
```

## Project Structure
- `src/main.rs` - Application entry point, command-line argument parsing
- `src/archive.rs` - Core archiving functionality, image downloading
- `src/bluesky.rs` - Bluesky API integration, post fetching
- `src/database.rs` - SQLite database operations, duplicate tracking
- `archive/` - Storage directory for archived data
- `Dockerfile` & `docker-compose.yml` - Container configuration

## Bluesky API Integration

### Key Endpoints Used
- `com.atproto.server.createSession` - Authentication
- `app.bsky.feed.getActorLikes` - Fetch liked posts
- `app.bsky.feed.getAuthorFeed` - Fetch user's posts (with `filter=posts_with_media`)
- `com.atproto.sync.getBlob` - Download image blobs

### Post Filtering
When archiving user posts, the code filters out:
- **Reposts**: Posts with a `reason` field in the feed item
- **Quote posts**: Posts where embed type contains "record"
- **Non-image posts**: Only includes posts with `Embed::Images` type

### Rate Limiting
- Automatic retry with exponential backoff (up to 5 retries)
- Optional delay between requests (`--delay` flag)
- Progress bars with ETA using `indicatif` crate

## Database Schema
SQLite database tracks:
- `archived_posts`: Post metadata (URI, CID, author, text, timestamps)
- `archived_images`: Image metadata (blob CID, filename, MIME type, size, alt text)
- Uses blob CID to prevent duplicate downloads

## Adding New Features

### Example: Adding User Archive Feature
1. Add command-line argument in `main.rs`
2. Implement API function in `bluesky.rs` (follow existing patterns)
3. Add necessary structs for API responses
4. Update main logic to handle new option
5. Reuse existing archiving infrastructure
6. Update README documentation

### Common Patterns
- Use `get_*_with_options` naming for flexible API functions
- Include cursor support for pagination
- Add progress bars for long operations
- Handle rate limiting gracefully
- Filter posts at the API level when possible

## Git Workflow
```bash
# Commit with detailed message
git commit -m "feat: Description of feature

- Detail 1
- Detail 2

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-Authored-By: Claude <noreply@anthropic.com>"

# Push to main
git push origin main
```

## Common Issues & Solutions

### Compilation Warnings
- Many struct fields are for deserialization only (marked with `#[warn(dead_code)]`)
- This is normal for API response structs

### Docker Build
- Uses multi-stage build for smaller images
- Warning about BLUESKY_APP_PASSWORD in ENV is expected (used for runtime config)

### API Quirks
- `posts_with_media` filter helps reduce data transfer
- Feed responses may include various post types (check embed types)
- Always check for `None` cursors to detect end of pagination

## Development Guidelines
- Follow Rust best practices and idioms
- Use error handling with Result types
- Keep functions focused and testable
- Document public APIs
- Preserve exact indentation when editing files
- Check existing patterns before implementing new features
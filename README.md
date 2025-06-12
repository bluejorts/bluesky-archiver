# Bluesky Archiver

A command-line tool to archive image posts from Bluesky. You can archive images from your liked posts or from a specific user's timeline. The tool tracks previously downloaded images to avoid duplicates, making it suitable for scheduled runs.

## Features

- Downloads all images from liked posts or from a specific user's timeline
- Filters out reposts and quote posts when archiving user timelines
- Tracks downloaded images in SQLite database to avoid re-downloading
- Organizes images by author handle
- Automatically separates NSFW/content warning posts to a separate directory
- Option to archive only NSFW content
- Supports authentication via app passwords
- Configurable download limits
- Detailed logging
- Progress bars with ETA and speed metrics
- Optimized for large archives (handles unlimited posts efficiently)

## Installation

### Using Docker (Recommended)

The easiest way to run Bluesky Archiver is using the pre-built Docker image:

```bash
# Create archive directory with proper permissions
mkdir -p archive && chmod 777 archive

# Run with docker
docker run -v ./archive:/archive \
  -e BLUESKY_USERNAME=your-username \
  -e BLUESKY_APP_PASSWORD=your-app-password \
  polymetric/bluesky-archiver:latest
```

### Using Docker Compose

Create a `docker-compose.yml` file:

```yaml
version: '3.8'

services:
  bluesky-archiver:
    image: polymetric/bluesky-archiver:latest
    environment:
      BLUESKY_USERNAME: "your-username"  # Without @
      BLUESKY_APP_PASSWORD: "your-app-password"  # Use app password, not main password
      BLUESKY_LIMIT: "0"  # 0 = unlimited
      BLUESKY_DELAY: "100"  # Milliseconds between API requests
    volumes:
      - ./archive:/archive
```

Then run:
```bash
docker-compose up
```

### Building from Source

1. Make sure you have Rust installed (https://rustup.rs/)
2. Clone this repository
3. Build the project:

```bash
cargo build --release
```

The binary will be available at `target/release/bluesky-archiver`

## Usage

### Archive your liked posts
```bash
bluesky-archiver --username YOUR_USERNAME --password YOUR_APP_PASSWORD
```

### Archive all images from a specific user
To archive all image posts from a specific user (excluding reposts and quote posts):
```bash
bluesky-archiver --username YOUR_USERNAME --password YOUR_APP_PASSWORD --archive-user TARGET_USER
```

### Command Line Options

- `-u, --username <USERNAME>`: Your Bluesky username (without @)
- `-p, --password <PASSWORD>`: Your Bluesky app password (NOT your main password)
- `-o, --output <PATH>`: Directory to save images (default: ./archive)
- `-l, --limit <NUMBER>`: Maximum posts to fetch per run (default: 100, use 0 for unlimited)
- `-v, --verbose`: Enable verbose logging
- `--nsfw-only`: Only archive posts with NSFW/content warning labels
- `-d, --delay <DELAY>`: Delay between API requests in milliseconds (helps avoid rate limits)
- `--resume`: Resume from last saved position (useful for large archives)
- `--archive-user <USERNAME>`: Archive all image posts from a specific user (without @)

### Environment Variables

When using Docker, you can configure the archiver using the following environment variables:

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `BLUESKY_USERNAME` | Yes | - | Your Bluesky username (without @) |
| `BLUESKY_APP_PASSWORD` | Yes | - | Your Bluesky app password (NOT your main password) |
| `BLUESKY_OUTPUT` | No | `/archive` | Directory to save archived images |
| `BLUESKY_LIMIT` | No | `0` | Maximum posts to fetch per run (0 = unlimited) |
| `BLUESKY_DELAY` | No | `100` | Delay between API requests in milliseconds |
| `BLUESKY_VERBOSE` | No | `false` | Enable verbose/debug logging |
| `BLUESKY_NSFW_ONLY` | No | `false` | Only archive posts with NSFW/content warning labels |
| `BLUESKY_RESUME` | No | `false` | Resume from last saved position |

Example Docker run with all options:
```bash
docker run -v ./archive:/archive \
  -e BLUESKY_USERNAME=your-username \
  -e BLUESKY_APP_PASSWORD=your-app-password \
  -e BLUESKY_OUTPUT=/archive \
  -e BLUESKY_LIMIT=1000 \
  -e BLUESKY_DELAY=200 \
  -e BLUESKY_VERBOSE=true \
  -e BLUESKY_NSFW_ONLY=false \
  -e BLUESKY_RESUME=true \
  polymetric/bluesky-archiver:latest
```

For standalone usage, you can set the app password as an environment variable:

```bash
export BLUESKY_APP_PASSWORD="your-app-password"
bluesky-archiver -u your.username
```

## Getting an App Password

1. Log into your Bluesky account
2. Go to Settings → App Passwords
3. Create a new app password
4. Use this password with the tool (never use your main password)

## File Organization

Images are saved in the following structure:
```
archive/
├── archive.db          # SQLite database tracking downloads
├── username1/          # Regular content
│   ├── username1_2024-01-15T10-30-00_abc123_0.jpg
│   └── username1_2024-01-15T11-45-00_def456_0.png
├── username2/
│   └── username2_2024-01-14T09-00-00_ghi789_0.jpg
└── nsfw/              # NSFW/content warning posts
    ├── username1/
    │   └── username1_2024-01-16T14-20-00_xyz789_0.jpg
    └── username3/
        └── username3_2024-01-17T09-15-00_mno456_0.png
```

Filename format: `{author}_{timestamp}_{post-id}_{index}.{ext}`

Posts with NSFW or content warning labels (porn, sexual, nudity, graphic-media, self-harm, sensitive, content-warning) are automatically separated into the `nsfw/` subdirectory.

## Database Schema

The tool uses SQLite to track:
- Archived posts (URI, author, text, timestamps)
- Downloaded images (filename, size, alt text, download time)

## Handling Rate Limits

When archiving large numbers of posts (especially with `-l 0`), you may encounter rate limits. The tool handles this automatically:

1. **Automatic retries**: Detects rate limits and retries with exponential backoff
2. **Delay option**: Use `-d 100` to add a 100ms delay between requests
3. **Resume capability**: Use `--resume` to continue from where you left off if interrupted

Example for large archives:
```bash
# First run with delay to avoid rate limits
bluesky-archiver -u your.username -p your-app-password -l 0 -d 100

# If interrupted, resume from last position
bluesky-archiver -u your.username -p your-app-password -l 0 -d 100 --resume
```

## GitHub Actions Setup

This project includes GitHub Actions workflows for CI/CD. To enable integration tests in your fork:

1. Go to your repository Settings → Secrets and variables → Actions
2. Add the following repository secrets:
   - `BLUESKY_USERNAME`: Your Bluesky username (without @)
   - `BLUESKY_PASSWORD`: Your Bluesky app password (not your main password)

The integration tests will automatically run when:
- You push to the main branch
- You create a pull request from within the same repository

Integration tests are skipped for pull requests from forks for security reasons.

## Scheduling

To run this tool on a schedule, you can use cron (Linux/Mac) or Task Scheduler (Windows).

### With Docker

Example cron entry to run every 6 hours using Docker:
```bash
0 */6 * * * docker run -v /path/to/archive:/archive -e BLUESKY_USERNAME=your-username -e BLUESKY_APP_PASSWORD=your-app-password polymetric/bluesky-archiver:latest
```

### With Binary

Example cron entry to run every 6 hours:
```bash
0 */6 * * * /path/to/bluesky-archiver -u your.username -p your-app-password
```

## Building from Source

```bash
# Clone the repository
git clone <repository-url>
cd bluesky-archiver

# Build in release mode
cargo build --release

# Run tests
cargo test

# Run with cargo
cargo run -- -u your.username -p your-app-password
```

## Testing

Run the test suite:

```bash
# Run all unit tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_database_creation

# Run integration tests (requires Bluesky credentials)
TEST_BLUESKY_USERNAME=your.handle TEST_BLUESKY_PASSWORD=your-app-password cargo test -- --ignored
```

### Test Coverage

The test suite includes:
- **Unit tests** for database operations, API client, and archive functionality
- **Integration tests** for full archive workflows (requires valid credentials)
- **Advanced tests** for NSFW filtering, duplicate detection, and statistics

## Development Setup

### Pre-commit Hooks

This project uses pre-commit hooks to ensure code quality. To set up:

```bash
# Install pre-commit
pip install pre-commit

# Install the git hooks
pre-commit install

# Run against all files (optional)
pre-commit run --all-files
```

The pre-commit hooks will automatically:
- Format code with `cargo fmt`
- Run `cargo clippy` with warnings as errors
- Run all tests with `cargo test`
- Build the project with `cargo build`
- Fix trailing whitespace and file endings
- Validate YAML and TOML files

## License

MIT

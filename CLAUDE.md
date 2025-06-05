# CLAUDE.md - Project Context for AI Assistants

## Project Overview
Bluesky Archiver - A Rust application for archiving Bluesky social media posts and data.

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
```

## Project Structure
- `src/main.rs` - Application entry point
- `src/archive.rs` - Core archiving functionality
- `src/bluesky.rs` - Bluesky API integration
- `src/database.rs` - Database operations
- `archive/` - Storage directory for archived data
- `Dockerfile` & `docker-compose.yml` - Container configuration

## Environment Variables
See README.md for comprehensive environment variable documentation.

## Development Guidelines
- Follow Rust best practices and idioms
- Use error handling with Result types
- Keep functions focused and testable
- Document public APIs
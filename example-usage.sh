#!/bin/bash

# Example usage of the Bluesky Archiver

# Build the project in release mode
echo "Building project..."
cargo build --release

# Example 1: Basic usage with username and password
echo "Example 1: Basic usage"
./target/release/bluesky-archiver --username your.username --password your-app-password

# Example 2: Using environment variable for password
echo "Example 2: Using environment variable"
export BLUESKY_APP_PASSWORD="your-app-password"
./target/release/bluesky-archiver --username your.username

# Example 3: Custom output directory and limit
echo "Example 3: Custom output and limit"
./target/release/bluesky-archiver \
    --username your.username \
    --password your-app-password \
    --output /path/to/archive \
    --limit 50

# Example 4: Verbose output for debugging
echo "Example 4: Verbose mode"
./target/release/bluesky-archiver \
    --username your.username \
    --password your-app-password \
    --verbose

# Example 5: Archive only NSFW content
echo "Example 5: NSFW only mode"
./target/release/bluesky-archiver \
    --username your.username \
    --password your-app-password \
    --nsfw-only

# Example 6: Scheduled execution with cron
echo "Example 6: Add to crontab for scheduled runs"
echo "0 */6 * * * BLUESKY_APP_PASSWORD='your-app-password' /path/to/bluesky-archiver -u your.username"

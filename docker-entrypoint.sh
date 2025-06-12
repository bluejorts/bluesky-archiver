#!/bin/sh
set -e

# Build command with environment variables
CMD="/usr/local/bin/bluesky-archiver"

# Required arguments
if [ -n "$BLUESKY_USERNAME" ]; then
    CMD="$CMD --username $BLUESKY_USERNAME"
else
    echo "Error: BLUESKY_USERNAME is required"
    exit 1
fi

# Output directory
CMD="$CMD --output $BLUESKY_OUTPUT"

# Password is handled by clap env feature automatically
# Just need to ensure BLUESKY_APP_PASSWORD is set
if [ -z "$BLUESKY_APP_PASSWORD" ]; then
    echo "Error: BLUESKY_APP_PASSWORD is required"
    exit 1
fi

# Optional arguments
CMD="$CMD --limit $BLUESKY_LIMIT"
CMD="$CMD --delay $BLUESKY_DELAY"

# Boolean flags
if [ "$BLUESKY_VERBOSE" = "true" ]; then
    CMD="$CMD --verbose"
fi

if [ "$BLUESKY_NSFW_ONLY" = "true" ]; then
    CMD="$CMD --nsfw-only"
fi

if [ "$BLUESKY_RESUME" = "true" ]; then
    CMD="$CMD --resume"
fi

# Execute the command
exec $CMD "$@"

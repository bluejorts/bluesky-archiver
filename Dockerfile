# Build stage
FROM rust:1.82-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

# Create app directory
WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock* ./

# Create dummy main.rs to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release

# Copy source code
COPY src ./src

# Build the actual application
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates

# Create non-root user
RUN addgroup -g 1000 appuser && \
    adduser -D -s /bin/sh -u 1000 -G appuser appuser

# Copy binary from builder
COPY --from=builder /app/target/release/bluesky-archiver /usr/local/bin/bluesky-archiver

# Create archive directory
RUN mkdir -p /archive && chown appuser:appuser /archive

# Environment variables matching CLI arguments
ENV BLUESKY_USERNAME=""
ENV BLUESKY_OUTPUT="/archive"
ENV BLUESKY_APP_PASSWORD=""
ENV BLUESKY_LIMIT="0"
ENV BLUESKY_VERBOSE="false"
ENV BLUESKY_NSFW_ONLY="false"
ENV BLUESKY_DELAY="100"
ENV BLUESKY_RESUME="false"

# Copy entrypoint script
COPY docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Switch to non-root user
USER appuser

# Set working directory
WORKDIR /archive

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
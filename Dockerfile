# Multi-stage build for optimized Rust binary
# Stage 1: Build the application
FROM rust:1.83-slim as builder

# Install required dependencies for sqlx and PostgreSQL
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY . .

# Touch main.rs to ensure it gets rebuilt
RUN touch src/main.rs

# Build the actual application
RUN cargo build --release

# Stage 2: Create minimal runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/bigtwo /app/bigtwo

# Copy migrations for production use
COPY --from=builder /app/migrations /app/migrations

# Expose port (Railway will use PORT env var)
EXPOSE 3000

# Run the binary
CMD ["/app/bigtwo"]

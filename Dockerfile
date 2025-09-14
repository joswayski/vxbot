# Use the official Rust image as the base image
FROM rust:latest AS builder

# Set the working directory inside the container
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies first (for better caching)
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached if Cargo.toml doesn't change)
RUN cargo build --release && rm src/main.rs

# Copy the source code
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage - use a smaller base image
FROM debian:bookworm-slim

# Install required runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -r -s /bin/false vxbot

# Set the working directory
WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /app/target/release/vxbot /app/vxbot

# Change ownership to the non-root user
RUN chown vxbot:vxbot /app/vxbot

# Switch to the non-root user
USER vxbot

# Expose any necessary ports (Discord bots typically don't need exposed ports)
# EXPOSE 8080

# Set the entrypoint
ENTRYPOINT ["./vxbot"]

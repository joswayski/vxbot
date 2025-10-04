# Use the official Rust image as the base image
FROM rust:latest AS builder

# Set the working directory inside the container
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies first (for better caching)
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached if Cargo.toml doesn't change)
# IMPORTANT: We must remove not just the dummy source file, but also the compiled
# dummy binary and its dependencies. Otherwise, when we copy the real source code
# and rebuild, Cargo may think the binary is already up-to-date and skip recompiling,
# resulting in the dummy "hello world" binary being used instead of our actual bot code.
RUN cargo build --release && rm -rf src target/release/vxbot* target/release/deps/vxbot*

# Copy the actual source code
COPY src ./src

# Build the actual application (now guaranteed to recompile with the real source)
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

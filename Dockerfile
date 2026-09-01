# ==============================================================================
# COYWIN GENERATIVE NODE - RAILWAY COMPLIANT DEPLOYMENT
# ==============================================================================

# Phase 1: The Builder Stage
FROM rust:latest AS builder
WORKDIR /usr/src/coywin

# Copy workspace and source files
COPY . .

# Build the release binary
RUN cargo build --release -p coywin-node

# Phase 2: Lightweight Runtime Image
FROM debian:bookworm-slim
WORKDIR /app

# Install standard runtime SSL and network certificates
RUN apt-get update && apt-get install -y libssl3 ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Copy compiled executable from builder
COPY --from=builder /usr/src/coywin/target/release/coywin-node /usr/local/bin/coywin-node

# Ensure runtime directories exist
RUN mkdir -p /app/output_images

# Railway injects $PORT dynamically, default to 8080
ENV PORT=8080
ENV HOST=0.0.0.0
ENV OUTPUT_DIR=/app/output_images

EXPOSE 8080

CMD ["coywin-node"]

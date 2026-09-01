# ==============================================================================
# COYWIN GENERATIVE NODE - DOCKER RUNTIME
# Compatible with Hugging Face Spaces, Railway, Render, and VPS
# ==============================================================================

# Phase 1: Builder Stage
FROM rust:latest AS builder
WORKDIR /usr/src/coywin
COPY . .
RUN cargo build --release -p coywin-node

# Phase 2: Lightweight Runtime
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update && apt-get install -y libssl3 ca-certificates curl && rm -rf /var/lib/apt/lists/*

# Create non-root user (UID 1000) for Hugging Face Spaces compatibility
RUN useradd -m -u 1000 user && \
    mkdir -p /app/output_images && \
    chown -R user:user /app

COPY --from=builder /usr/src/coywin/target/release/coywin-node /usr/local/bin/coywin-node

USER user
ENV PORT=8080
ENV HOST=0.0.0.0
ENV OUTPUT_DIR=/app/output_images

EXPOSE 8080

CMD ["coywin-node"]

---
title: Coywin Generative Node
emoji: 🦀
colorFrom: indigo
colorTo: green
sdk: docker
app_port: 8080
pinned: false
---

# Coywin Generative Node & Block API

Coywin is a high-performance Rust node engine combining ML-DSA-87 (Dilithium5) post-quantum signatures with a deterministic vector rasterizer to produce generative visual artwork blocks.

Designed for modern cloud deployments (Hugging Face Spaces, Railway, Render, VPS) as an asynchronous HTTP REST API and decentralized peer service without cryptocurrency mining loops.

## Architecture

The engine workspace consists of modular crates:

* `coywin-node`: The async HTTP REST API service and P2P gossipsub engine powered by `tokio`, `axum`, and `libp2p`.
* `coywin-render`: A deterministic 2.5D software rasterizer producing 1920x1080 visual representations of cryptographic block states.
* `coywin-steg`: Prime-grid LSB steganography engine embedding data payloads with AVX2/AVX-512 acceleration.
* `coywin-ledger`: Validates gift transactions with `pqcrypto-dilithium` and manages state persistence with `sled`.

## HTTP API Endpoints

The node listens on port `8080` and exposes the following REST API endpoints:

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/` | Web dashboard & live block viewer |
| `GET` | `/health` | Service health status |
| `GET` | `/node/info` | Node metrics, peer counts, and protocol info |
| `GET` | `/block/latest` | Latest generated block in JSON format |
| `GET` | `/block/:id` | Lookup block by height, hash, or phonetic name |
| `POST` | `/block/generate` | On-demand deterministic block synthesis |
| `GET` | `/api/images` | JSON gallery feed of rendered matrix images |
| `GET` | `/output_images/*` | Static file server for rendered block PNGs |

### Example Block Output (`GET /block/latest`)

```json
{
  "height": 1,
  "hash": "a4f89d3c872e01b4c9e82103f6d7a2b5...",
  "phonetic_name": "Fihekilfong",
  "timestamp": 1788231234,
  "nonce": 42,
  "generator": "coywin-deterministic-api-engine",
  "pqc_secured": true,
  "image_path": "output_images/Fihekilfong_a4f89d3c.png",
  "image_url": "/output_images/Fihekilfong_a4f89d3c.png",
  "matrix_width": 1920,
  "matrix_height": 1080,
  "summary": "Generative block produced via API request."
}
```

## Running Locally

```bash
cargo run --release -p coywin-node
```

Visit `http://localhost:8080` in your web browser.

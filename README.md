---
title: Coywin Generative Node
emoji: 🦀
colorFrom: indigo
colorTo: green
sdk: gradio
app_file: app.py
pinned: false
---

# Coywin Generative Node & Block API

Coywin is a vector art matrix engine producing post-quantum secured generative visual blocks.

## REST API Endpoints
- `GET /health` : Health check status
- `GET /node/info` : Node metrics & compliance info
- `GET /block/latest` : Latest generated block (JSON)
- `POST /block/generate` : On-demand block synthesis

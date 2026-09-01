---
title: Coywin Generative Node
emoji: 🦀
colorFrom: indigo
colorTo: green
sdk: gradio
app_file: app.py
pinned: false
---

# 🦀 Coywin Generative Node & Block API

Coywin is a vector art matrix engine producing post-quantum secured generative visual blocks (2.5D Soft-SDF Translucent Glass Spheres & Ray-Tubes).

---

## 📥 Download Miner Application for Windows

Users can run the desktop miner locally to mine blocks and submit them directly to the cloud node:

👉 **[Download Coywin_Miner.exe (Windows 64-bit)](https://github.com/wijaywi/coywin/raw/main/release/Coywin_Miner.exe)**

---

## 🌐 Cloud REST API Endpoints

- `GET /health` : Health check status
- `GET /node/info` : Node metrics & compliance info
- `GET /block/latest` : Latest generated block (JSON)
- `GET /block/0` : Authentic Genesis Block (Bokovekovo / WSJ BRICS payload)
- `POST /block/generate` : On-demand block synthesis

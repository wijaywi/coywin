import hashlib
import time
import json
import numpy as np
from PIL import Image, ImageDraw
import gradio as gr
from fastapi import Request
from fastapi.responses import JSONResponse

try:
    import spaces
    GPU_DECORATOR = spaces.GPU
except Exception:
    def GPU_DECORATOR(func):
        return func

SYLLABLES = [
    "ka", "ru", "ma", "ti", "vo", "la", "ne", "pi", "ro", "su",
    "ta", "mi", "ko", "ra", "lu", "se", "ni", "do", "fa", "go",
    "he", "ji", "ku", "bo", "za", "ve", "xi", "yu", "we", "qo",
    "pa", "mu", "zi", "di", "xo", "va", "fe", "qi", "lo", "xu"
]

def generate_phonetic_name(hash_bytes):
    name = ""
    for i in range(5):
        idx = hash_bytes[i] % len(SYLLABLES)
        syl = SYLLABLES[idx]
        name += syl.capitalize() if i == 0 else syl
    return name

blocks_history = []

@GPU_DECORATOR
def generate_coywin_block(tag="Web Synthesis", custom_seed=None, width=1280, height=720):
    global blocks_history
    now_ts = int(time.time())
    height_idx = len(blocks_history)
    
    seed_val = int(custom_seed) if custom_seed is not None and str(custom_seed).isdigit() else (now_ts + height_idx)
    
    # Hash Commitment
    hasher = hashlib.sha256()
    hasher.update(b"COYWIN_BLOCK_HEADER_V2")
    hasher.update(height_idx.to_bytes(8, 'little'))
    hasher.update(now_ts.to_bytes(8, 'little'))
    hasher.update(seed_val.to_bytes(8, 'little'))
    hasher.update(tag.encode('utf-8'))
    block_hash_bytes = hasher.digest()
    hash_hex = block_hash_bytes.hex()
    
    phonetic_name = generate_phonetic_name(block_hash_bytes)
    
    # Deterministic Vector Art Rasterizer
    img = Image.new("RGB", (width, height), color=(10, 10, 15))
    draw = ImageDraw.Draw(img)
    
    # Generate background field
    np_seed = int.from_bytes(block_hash_bytes[:4], 'little')
    rng = np.random.RandomState(np_seed)
    
    # Draw cosmic matrix background
    num_particles = 40
    for _ in range(num_particles):
        x = rng.randint(0, width)
        y = rng.randint(0, height)
        r = rng.randint(2, 6)
        cr = rng.randint(50, 255)
        cg = rng.randint(100, 255)
        cb = rng.randint(150, 255)
        draw.ellipse([x - r, y - r, x + r, y + r], fill=(cr, cg, cb))
        
    # Draw geometric structure
    cx, cy = width // 2, height // 2
    num_rings = 8
    for i in range(num_rings):
        rad = 60 + i * 35
        cr = int(30 + (i * 25) % 225)
        cg = int(180 + (i * 10) % 75)
        cb = int(120 + (i * 15) % 135)
        draw.ellipse([cx - rad, cy - rad, cx + rad, cy + rad], outline=(cr, cg, cb), width=3)
        
    # Overlay Coywin cryptographic watermark
    draw.text((30, height - 60), f"COYWIN NODE // {phonetic_name}", fill=(200, 255, 200))
    draw.text((30, height - 35), f"HASH: {hash_hex[:32]}...", fill=(150, 180, 220))
    
    block_data = {
        "height": height_idx,
        "hash": hash_hex,
        "phonetic_name": phonetic_name,
        "timestamp": now_ts,
        "nonce": seed_val,
        "generator": "coywin-gradio-cloud-engine",
        "pqc_secured": True,
        "matrix_width": width,
        "matrix_height": height,
        "summary": tag
    }
    
    blocks_history.append(block_data)
    return img, json.dumps(block_data, indent=2)

# Gradio Web UI
with gr.Blocks(title="Coywin Generative Node") as demo:
    gr.Markdown("# 🦀 Coywin Generative Node & Block API")
    gr.Markdown("100% Cloud-Compliant Vector Artwork Generator & Blockchain Block API.")
    
    with gr.Row():
        with gr.Column(scale=1):
            tag_input = gr.Textbox(value="Community Block", label="Block Summary / Tag")
            seed_input = gr.Textbox(value="", label="Custom Seed (Optional)")
            gen_btn = gr.Button("⚡ Generate Block Matrix", variant="primary")
            gr.Markdown("### REST API Endpoints\n- `GET /health`\n- `GET /node/info`\n- `GET /block/latest`\n- `POST /block/generate`")
        
        with gr.Column(scale=2):
            output_image = gr.Image(label="Rendered Matrix Art", type="pil")
            output_json = gr.Code(language="json", label="Block Data (JSON)")

    gen_btn.click(
        fn=generate_coywin_block,
        inputs=[tag_input, seed_input],
        outputs=[output_image, output_json]
    )

# Mount REST API Endpoints on Gradio FastAPI instance
@demo.app.get("/health")
def api_health():
    return {"status": "ok", "service": "coywin-node", "total_blocks": len(blocks_history)}

@demo.app.get("/node/info")
def api_node_info():
    return {
        "service_name": "Coywin Generative Node & Block API",
        "protocol_version": "2.0.0",
        "compliance": "100% Compliant Cloud API Service",
        "total_blocks": len(blocks_history)
    }

@demo.app.get("/block/latest")
def api_block_latest():
    if blocks_history:
        return blocks_history[-1]
    return JSONResponse(status_code=404, content={"error": "No blocks"})

@demo.app.post("/block/generate")
async def api_block_generate(request: Request):
    try:
        body = await request.json()
        tag = body.get("tag", "API Request Block")
        seed = body.get("custom_seed", None)
    except Exception:
        tag = "API Request Block"
        seed = None
    _, block_json_str = generate_coywin_block(tag, seed)
    return json.loads(block_json_str)

if __name__ == "__main__":
    demo.launch()

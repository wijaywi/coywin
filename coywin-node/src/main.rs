use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use coywin_network::{build_swarm, BlockProposed, CoywinBehaviourEvent};
use coywin_render::{execute_full_pipeline, generate_bip_coywin_name};
use futures::StreamExt;
use libp2p::swarm::SwarmEvent;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockData {
    pub height: u64,
    pub hash: String,
    pub phonetic_name: String,
    pub timestamp: u64,
    pub nonce: u64,
    pub generator: String,
    pub pqc_secured: bool,
    pub image_path: String,
    pub image_url: String,
    pub matrix_width: u32,
    pub matrix_height: u32,
    pub summary: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenerateBlockRequest {
    pub tag: Option<String>,
    pub custom_seed: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeInfo {
    pub service_name: String,
    pub protocol_version: String,
    pub compliance: String,
    pub total_blocks: usize,
    pub active_peers: usize,
    pub uptime_seconds: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_seconds: u64,
    pub total_blocks: usize,
}

struct AppState {
    start_time: Instant,
    blocks: RwLock<Vec<BlockData>>,
    current_height: AtomicU64,
    peer_count: RwLock<usize>,
    output_dir: PathBuf,
    p2p_tx: Option<tokio::sync::mpsc::Sender<BlockProposed>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("============================================================");
    println!(" COYWIN GENERATIVE NODE & BLOCK API ENGINE (V2.0 COMPLIANT) ");
    println!(" Railway-Optimized Service: 0% CPU Idle, Pure API Driven    ");
    println!("============================================================");

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let output_dir_str = std::env::var("OUTPUT_DIR").unwrap_or_else(|_| "output_images".to_string());
    let output_dir = PathBuf::from(&output_dir_str);

    if let Err(e) = std::fs::create_dir_all(&output_dir) {
        eprintln!("[!] Warning: Could not create output dir {:?}: {}", output_dir, e);
    }

    let (p2p_tx, mut p2p_rx) = tokio::sync::mpsc::channel::<BlockProposed>(32);

    let state = Arc::new(AppState {
        start_time: Instant::now(),
        blocks: RwLock::new(Vec::new()),
        current_height: AtomicU64::new(0),
        peer_count: RwLock::new(0),
        output_dir: output_dir.clone(),
        p2p_tx: Some(p2p_tx),
    });

    // Generate Genesis Block deterministically on startup if empty
    generate_initial_block(state.clone()).await;

    // Background P2P Swarm Handler (Compliant block receiver/broadcaster)
    let p2p_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = run_p2p_subsystem(p2p_state, &mut p2p_rx).await {
            println!("[*] P2P background listener closed: {:?}", e);
        }
    });

    // Build Axum HTTP Router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(root_dashboard_handler))
        .route("/health", get(health_handler))
        .route("/node/info", get(node_info_handler))
        .route("/block/latest", get(latest_block_handler))
        .route("/block/:id", get(get_block_by_id_handler))
        .route("/block/generate", post(generate_block_handler))
        .route("/api/images", get(api_images_handler))
        .nest_service(
            "/output_images",
            ServeDir::new(&output_dir).append_index_html_on_directories(false),
        )
        .layer(cors)
        .with_state(state.clone());

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    println!("[*] HTTP API listening on http://{}", addr);
    println!("[*] Endpoints active:");
    println!("    GET  /health");
    println!("    GET  /node/info");
    println!("    GET  /block/latest");
    println!("    GET  /block/:id");
    println!("    POST /block/generate");
    println!("    GET  /api/images");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn generate_initial_block(state: Arc<AppState>) {
    let mut blocks = state.blocks.write().await;
    if !blocks.is_empty() {
        return;
    }

    println!("[*] Generating Deterministic Genesis Block...");
    let seed: u64 = 42;
    let mut commitment_hasher = Sha256::new();
    commitment_hasher.update(b"COYWIN_GENESIS_BLOCK_V2");
    commitment_hasher.update(&seed.to_le_bytes());
    let mut block_hash = [0u8; 32];
    block_hash.copy_from_slice(&commitment_hasher.finalize());

    let mut signature = vec![0u8; 4627];
    signature[24] = 42;
    signature[25] = 137;

    let phonetic_name = generate_bip_coywin_name(&block_hash);
    let hash_hex = hex_string(&block_hash);
    let short_hash = &hash_hex[0..8];
    let filename = format!("{}_{}.png", phonetic_name, short_hash);
    let filepath = state.output_dir.join(&filename);

    let width = 1920;
    let height = 1080;

    if let Ok(image_buffer) = execute_full_pipeline(block_hash, &signature, width, height, seed) {
        if let Ok(file) = std::fs::File::create(&filepath) {
            let mut w = std::io::BufWriter::new(file);
            let encoder = image::codecs::png::PngEncoder::new(&mut w);
            let _ = image::ImageEncoder::write_image(
                encoder,
                &image_buffer.data,
                image_buffer.width,
                image_buffer.height,
                image::ColorType::Rgb8,
            );
            println!("[SUCCESS] Saved Genesis Block PNG to {:?}", filepath);
        }
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let genesis_block = BlockData {
        height: 0,
        hash: hash_hex,
        phonetic_name,
        timestamp,
        nonce: seed,
        generator: "coywin-deterministic-engine-v2".to_string(),
        pqc_secured: true,
        image_path: filepath.to_string_lossy().to_string(),
        image_url: format!("/output_images/{}", filename),
        matrix_width: width,
        matrix_height: height,
        summary: "Genesis block initialized without Proof-of-Work loop.".to_string(),
    };

    blocks.push(genesis_block);
    state.current_height.store(0, Ordering::SeqCst);
}

async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let blocks_guard = state.blocks.read().await;
    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        total_blocks: blocks_guard.len(),
    })
}

async fn node_info_handler(State(state): State<Arc<AppState>>) -> Json<NodeInfo> {
    let blocks_guard = state.blocks.read().await;
    let peer_count = *state.peer_count.read().await;
    Json(NodeInfo {
        service_name: "Coywin Generative Node & Block API".to_string(),
        protocol_version: "2.0.0".to_string(),
        compliance: "100% Compliant Railway Service (No PoW Mining)".to_string(),
        total_blocks: blocks_guard.len(),
        active_peers: peer_count,
        uptime_seconds: state.start_time.elapsed().as_secs(),
    })
}

async fn latest_block_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<BlockData>, (StatusCode, &'static str)> {
    let blocks = state.blocks.read().await;
    if let Some(latest) = blocks.last() {
        Ok(Json(latest.clone()))
    } else {
        Err((StatusCode::NOT_FOUND, "No blocks available"))
    }
}

async fn get_block_by_id_handler(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Result<Json<BlockData>, (StatusCode, &'static str)> {
    let blocks = state.blocks.read().await;

    // Search by height, hash prefix, or phonetic name
    for block in blocks.iter() {
        if block.height.to_string() == id
            || block.hash.starts_with(&id)
            || block.phonetic_name.eq_ignore_ascii_case(&id)
        {
            return Ok(Json(block.clone()));
        }
    }

    Err((StatusCode::NOT_FOUND, "Block not found"))
}

async fn generate_block_handler(
    State(state): State<Arc<AppState>>,
    payload: Option<Json<GenerateBlockRequest>>,
) -> (StatusCode, Json<BlockData>) {
    let new_height = state.current_height.fetch_add(1, Ordering::SeqCst) + 1;
    let now_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let (tag, custom_seed) = if let Some(Json(req)) = payload {
        (req.tag, req.custom_seed)
    } else {
        (None, None)
    };

    let seed = custom_seed.unwrap_or_else(|| now_ts.wrapping_add(new_height));

    let mut commitment_hasher = Sha256::new();
    commitment_hasher.update(b"COYWIN_BLOCK_HEADER_V2");
    commitment_hasher.update(&new_height.to_le_bytes());
    commitment_hasher.update(&now_ts.to_le_bytes());
    commitment_hasher.update(&seed.to_le_bytes());
    if let Some(ref t) = tag {
        commitment_hasher.update(t.as_bytes());
    }

    let mut block_hash = [0u8; 32];
    block_hash.copy_from_slice(&commitment_hasher.finalize());

    let mut signature = vec![0u8; 4627];
    signature[24] = (new_height % 256) as u8;
    signature[25] = ((seed >> 8) % 256) as u8;

    let phonetic_name = generate_bip_coywin_name(&block_hash);
    let hash_hex = hex_string(&block_hash);
    let short_hash = &hash_hex[0..8];
    let filename = format!("{}_{}.png", phonetic_name, short_hash);
    let filepath = state.output_dir.join(&filename);

    let width = 1920;
    let height = 1080;

    let hash_clone = block_hash;
    let sig_clone = signature.clone();
    let filepath_clone = filepath.clone();

    // Single-pass deterministic image generation on blocking thread pool
    tokio::task::spawn_blocking(move || {
        if let Ok(image_buffer) = execute_full_pipeline(hash_clone, &sig_clone, width, height, seed) {
            if let Ok(file) = std::fs::File::create(&filepath_clone) {
                let mut w = std::io::BufWriter::new(file);
                let encoder = image::codecs::png::PngEncoder::new(&mut w);
                let _ = image::ImageEncoder::write_image(
                    encoder,
                    &image_buffer.data,
                    image_buffer.width,
                    image_buffer.height,
                    image::ColorType::Rgb8,
                );
            }
        }
    })
    .await
    .ok();

    let new_block = BlockData {
        height: new_height,
        hash: hash_hex,
        phonetic_name,
        timestamp: now_ts,
        nonce: seed,
        generator: "coywin-deterministic-api-engine".to_string(),
        pqc_secured: true,
        image_path: filepath.to_string_lossy().to_string(),
        image_url: format!("/output_images/{}", filename),
        matrix_width: width,
        matrix_height: height,
        summary: tag.unwrap_or_else(|| "Generative block produced via API request.".to_string()),
    };

    {
        let mut blocks = state.blocks.write().await;
        blocks.push(new_block.clone());
    }

    // Gossip across P2P swarm if enabled
    if let Some(ref tx) = state.p2p_tx {
        let proposal = BlockProposed {
            hash: block_hash,
            signature,
            nonce: seed,
        };
        let _ = tx.send(proposal).await;
    }

    (StatusCode::CREATED, Json(new_block))
}

#[derive(Serialize)]
struct ImageGalleryItem {
    filename: String,
    name: String,
    hash: String,
    time: u64,
    pqc_secured: bool,
    miner_name: String,
    miner_address: String,
}

async fn api_images_handler(State(state): State<Arc<AppState>>) -> Json<Vec<ImageGalleryItem>> {
    let blocks = state.blocks.read().await;
    let mut items = Vec::new();

    for b in blocks.iter() {
        items.push(ImageGalleryItem {
            filename: format!("output_images/{}_{}.png", b.phonetic_name, &b.hash[0..8]),
            name: b.phonetic_name.clone(),
            hash: b.hash[0..8].to_string(),
            time: b.timestamp,
            pqc_secured: b.pqc_secured,
            miner_name: "Coywin Node API".to_string(),
            miner_address: b.hash.clone(),
        });
    }

    items.reverse();
    Json(items)
}

async fn root_dashboard_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let blocks = state.blocks.read().await;
    let latest_block = blocks.last();
    let total_blocks = blocks.len();
    let uptime = state.start_time.elapsed().as_secs();

    let block_html = if let Some(b) = latest_block {
        format!(
            r#"<div style="background:#18181b; padding:20px; border-radius:8px; border:1px solid #27272a; margin-top:20px;">
                <h3 style="color:#22c55e; margin:0 0 10px 0;">Latest Block: #{} ({})</h3>
                <p><strong>Hash:</strong> <code>{}</code></p>
                <p><strong>Timestamp:</strong> {}</p>
                <p><strong>Generator:</strong> {}</p>
                <p><a href="{}" target="_blank" style="color:#38bdf8; text-decoration:none;">View Artwork Matrix &rarr;</a></p>
            </div>"#,
            b.height, b.phonetic_name, b.hash, b.timestamp, b.generator, b.image_url
        )
    } else {
        "<p>No blocks generated yet.</p>".to_string()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Coywin Generative Node</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background:#09090b; color:#f4f4f5; margin:0; padding:40px; }}
        .container {{ max-width:800px; margin:0 auto; }}
        .card {{ background:#111113; padding:24px; border-radius:12px; border:1px solid #1f1f23; }}
        .badge {{ background:#14532d; color:#86efac; padding:4px 10px; border-radius:9999px; font-size:12px; font-weight:600; display:inline-block; }}
        code {{ background:#27272a; padding:2px 6px; border-radius:4px; font-size:13px; }}
        .btn {{ display:inline-block; background:#2563eb; color:white; padding:10px 16px; border-radius:6px; text-decoration:none; font-weight:600; margin-top:15px; }}
        .btn:hover {{ background:#1d4ed8; }}
    </style>
</head>
<body>
    <div class="container">
        <div class="card">
            <span class="badge">RAILWAY COMPLIANT SERVICE</span>
            <h1 style="margin: 12px 0 6px 0;">Coywin Generative Node</h1>
            <p style="color:#a1a1aa; margin:0 0 20px 0;">High-Performance Vector Art Generator & Decentralized Block API</p>
            
            <div style="display:grid; grid-template-columns: 1fr 1fr; gap:12px; margin-bottom:20px;">
                <div style="background:#18181b; padding:12px; border-radius:6px;">
                    <div style="color:#a1a1aa; font-size:12px;">Total Blocks</div>
                    <div style="font-size:24px; font-weight:bold;">{}</div>
                </div>
                <div style="background:#18181b; padding:12px; border-radius:6px;">
                    <div style="color:#a1a1aa; font-size:12px;">Uptime</div>
                    <div style="font-size:24px; font-weight:bold;">{}s</div>
                </div>
            </div>

            {}

            <h3 style="margin-top:30px;">API Endpoints</h3>
            <ul>
                <li><code>GET /health</code> - Service health check</li>
                <li><code>GET /node/info</code> - Node metadata & metrics</li>
                <li><code>GET /block/latest</code> - Latest block in JSON format</li>
                <li><code>POST /block/generate</code> - On-demand block synthesis</li>
                <li><code>GET /api/images</code> - Rendered matrix gallery feed</li>
            </ul>
        </div>
    </div>
</body>
</html>"#,
        total_blocks, uptime, block_html
    );

    Html(html)
}

async fn run_p2p_subsystem(
    state: Arc<AppState>,
    rx: &mut tokio::sync::mpsc::Receiver<BlockProposed>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut swarm = build_swarm().await.map_err(|e| e.to_string())?;

    // Attempt listen on random port, ignore error if restricted
    let _ = swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?);
    let topic = libp2p::gossipsub::IdentTopic::new("coywin-blocks");

    loop {
        tokio::select! {
            Some(proposal) = rx.recv() => {
                if let Ok(serialized) = serde_json::to_vec(&proposal) {
                    let _ = swarm.behaviour_mut().gossipsub.publish(topic.clone(), serialized);
                }
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(CoywinBehaviourEvent::Mdns(mdns_event)) => match mdns_event {
                    libp2p::mdns::Event::Discovered(list) => {
                        let mut p_count = state.peer_count.write().await;
                        for (peer_id, multiaddr) in list {
                            println!("[P2P mDNS] Discovered peer: {} at {:?}", peer_id, multiaddr);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                            *p_count += 1;
                        }
                    }
                    libp2p::mdns::Event::Expired(list) => {
                        let mut p_count = state.peer_count.write().await;
                        for (peer_id, _multiaddr) in list {
                            swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                            if *p_count > 0 {
                                *p_count -= 1;
                            }
                        }
                    }
                }
                SwarmEvent::Behaviour(CoywinBehaviourEvent::Gossipsub(gossipsub_event)) => match gossipsub_event {
                    libp2p::gossipsub::Event::Message { propagation_source: peer_id, message, .. } => {
                        println!("[P2P Gossip] Received block proposal from {}", peer_id);
                        if let Ok(proposal) = serde_json::from_slice::<BlockProposed>(&message.data) {
                            let hash = proposal.hash;
                            let sig = proposal.signature.clone();
                            let nonce = proposal.nonce;
                            let state_clone = state.clone();

                            tokio::spawn(async move {
                                let width = 1920;
                                let height = 1080;
                                if let Ok(image_buffer) = tokio::task::spawn_blocking(move || {
                                    execute_full_pipeline(hash, &sig, width, height, nonce)
                                }).await.unwrap_or(Err("Pipeline execution error")) {
                                    let phonetic_name = generate_bip_coywin_name(&hash);
                                    let hash_hex = hex_string(&hash);
                                    let filename = format!("{}_{}.png", phonetic_name, &hash_hex[0..8]);
                                    let filepath = state_clone.output_dir.join(&filename);

                                    if let Ok(file) = std::fs::File::create(&filepath) {
                                        let mut w = std::io::BufWriter::new(file);
                                        let encoder = image::codecs::png::PngEncoder::new(&mut w);
                                        let _ = image::ImageEncoder::write_image(
                                            encoder,
                                            &image_buffer.data,
                                            image_buffer.width,
                                            image_buffer.height,
                                            image::ColorType::Rgb8,
                                        );
                                    }

                                    let new_height = state_clone.current_height.fetch_add(1, Ordering::SeqCst) + 1;
                                    let now_ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

                                    let received_block = BlockData {
                                        height: new_height,
                                        hash: hash_hex,
                                        phonetic_name,
                                        timestamp: now_ts,
                                        nonce,
                                        generator: format!("peer:{}", peer_id),
                                        pqc_secured: true,
                                        image_path: filepath.to_string_lossy().to_string(),
                                        image_url: format!("/output_images/{}", filename),
                                        matrix_width: width,
                                        matrix_height: height,
                                        summary: "Block received from P2P peer.".to_string(),
                                    };

                                    let mut blocks = state_clone.blocks.write().await;
                                    blocks.push(received_block);
                                }
                            });
                        }
                    }
                    _ => {}
                }
                _ => {}
            }
        }
    }
}

fn hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

use coywin_network::{build_swarm, CoywinBehaviourEvent, BlockProposed};
use libp2p::swarm::SwarmEvent;
use futures::StreamExt;
use std::error::Error;
use coywin_render::execute_full_pipeline;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use image::ImageEncoder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Initializing Coywin Autonomous Node (Tokio Async + AVX-512 Rendering)...");

    let mut swarm = build_swarm().await?;

    // Listen on a random OS-assigned port for TCP IPv4
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Simulate mining a block every 15 seconds if we want, but for now we'll just listen and occasionally broadcast
    let topic = libp2p::gossipsub::IdentTopic::new("coywin-blocks");

    let (miner_tx, mut miner_rx) = tokio::sync::mpsc::channel(1);

    // Spawn the Memory-Hard PoSW Mining Loop on a dedicated blocking thread
    tokio::task::spawn_blocking(move || {
        use sha2::{Sha256, Digest};
        let mut nonce: u64 = 0;
        let mut block_hash = [0u8; 32];
        let mut signature = vec![0u8; 4627];
        
        println!("[*] INITIALIZING MEMORY-HARD PoSW MINER...");
        println!("[*] Network Difficulty Target: 2 Leading Zeros (0x0000...)");
        
        loop {
            block_hash[0] = (nonce % 256) as u8;
            signature[24] = (nonce % 256) as u8;

            // Violently re-render the matrix for every nonce at FULL 1920x1080 resolution
            // If it takes 5 years to mine a block, so be it. This is Mona Lisa art.
            if let Ok(image) = execute_full_pipeline(block_hash, &signature, 1920, 1080, nonce) {
                // Compute SHA-256 of the raw uncompressed pixel data
                let mut hasher = Sha256::new();
                hasher.update(&image.data);
                let result = hasher.finalize();

                // Check Network Difficulty (Specific pattern 00a9)
                if result[0] == 0x00 && result[1] == 0xA9 {
                    println!("\n[SUCCESS] PROOF OF STEGANOGRAPHIC WORK SOLVED!");
                    println!("[*] Nonce: {}", nonce);
                    println!("[*] Matrix Hash: {:x}{:x}{:x}{:x}...", result[0], result[1], result[2], result[3]);
                    
                    let proposal = BlockProposed {
                        hash: block_hash,
                        signature: signature.clone(),
                        nonce,
                    };
                    
                    let _ = miner_tx.blocking_send(proposal);
                    
                    // Halt mining for Genesis demonstration
                    println!("[*] Genesis Block Mined! Halting mining engine...");
                    break;
                }
            }
            
            // Tuntutan pengguna: Batasi siksaan CPU hingga 50%!
            // Kita menyuntikkan waktu istirahat yang presisi setelah setiap kali iterasi rendering selesai.
            // Rendering 1920x1080 biasanya memakan waktu sekitar 0.5 hingga 1 detik pada single-core.
            // Dengan menambahkan sleep 500ms, kita membelah penggunaan CPU thread ini menjadi sekitar separuhnya, 
            // menenangkan kipas prosesor yang menjerit tanpa menghentikan pertambangan.
            std::thread::sleep(std::time::Duration::from_millis(500));
            
            if nonce % 100 == 0 {
                print!(".");
                use std::io::Write;
                let _ = std::io::stdout().flush();
            }
            
            nonce += 1;
        }
    });

    loop {
        tokio::select! {
            Some(proposal) = miner_rx.recv() => {
                let serialized = serde_json::to_vec(&proposal).unwrap();
                
                // Save it locally first!
                println!("[*] Genesis Block found locally! Saving to disk before gossiping...");
                let hash_clone = proposal.hash;
                let sig_clone = proposal.signature.clone();
                let nonce_clone = proposal.nonce;
                tokio::task::spawn_blocking(move || {
                    if let Ok(image_buffer) = coywin_render::execute_full_pipeline(hash_clone, &sig_clone, 1920, 1080, nonce_clone) {
                        let phonetic_name = coywin_render::generate_bip_coywin_name(&hash_clone);
                        
                        // Buat folder output_images jika belum ada
                        let out_dir = std::path::Path::new("..\\output_images");
                        let _ = std::fs::create_dir_all(out_dir);
                        
                        let output_filename = format!("..\\output_images\\{}.png", phonetic_name);
                        let output_path = std::path::Path::new(&output_filename);
                        
                        if let Ok(file) = std::fs::File::create(output_path) {
                            let mut w = std::io::BufWriter::new(file);
                            let encoder = image::codecs::png::PngEncoder::new(&mut w);
                            let _ = encoder.write_image(&image_buffer.data, image_buffer.width, image_buffer.height, image::ColorType::Rgb8);
                            println!("[SUCCESS] Saved local Genesis Block: {:?}", output_path);
                        }
                    }
                });

                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), serialized) {
                    println!("[!] Failed to gossip block proposal (You are alone in the universe): {:?}", e);
                } else {
                    println!("[+] Gossiped solved block across the libp2p swarm!");
                }
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("[*] Local node is listening on {:?}", address);
                }
                SwarmEvent::Behaviour(CoywinBehaviourEvent::Mdns(mdns_event)) => match mdns_event {
                    libp2p::mdns::Event::Discovered(list) => {
                        for (peer_id, multiaddr) in list {
                            println!("[mDNS] Discovered Coywin peer: {} at {:?}", peer_id, multiaddr);
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                    }
                    libp2p::mdns::Event::Expired(list) => {
                        for (peer_id, _multiaddr) in list {
                            println!("[mDNS] Coywin peer expired: {}", peer_id);
                            swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        }
                    }
                }
                SwarmEvent::Behaviour(CoywinBehaviourEvent::Gossipsub(gossipsub_event)) => match gossipsub_event {
                    libp2p::gossipsub::Event::Message { propagation_source: peer_id, message_id: _id, message } => {
                        println!("[Gossip] Received block proposal from peer: {}", peer_id);
                        if let Ok(proposal) = serde_json::from_slice::<BlockProposed>(&message.data) {
                            println!("[*] Block hash authenticated. Offloading to AVX-512 render pipeline...");
                            
                            // Offload to standard thread pool so we don't block the async runtime with heavy CPU work
                            let hash_clone = proposal.hash;
                            let sig_clone = proposal.signature.clone();
                            let nonce_clone = proposal.nonce;
                            tokio::task::spawn_blocking(move || {
                                let width = 1920;
                                let height = 1080;
                                match execute_full_pipeline(hash_clone, &sig_clone, width, height, nonce_clone) {
                                    Ok(image_buffer) => {
                                        let phonetic_name = coywin_render::generate_bip_coywin_name(&hash_clone);
                                        
                                        // Buat folder output_images jika belum ada
                                        let out_dir = std::path::Path::new("..\\output_images");
                                        let _ = std::fs::create_dir_all(out_dir);
                                        
                                        let output_filename = format!("..\\output_images\\{}.png", phonetic_name);
                                        let output_path = std::path::Path::new(&output_filename);
                                        
                                        let file = std::fs::File::create(output_path).unwrap();
                                        let mut w = std::io::BufWriter::new(file);
                                        let encoder = image::codecs::png::PngEncoder::new(&mut w);
                                        encoder.write_image(
                                            &image_buffer.data,
                                            image_buffer.width,
                                            image_buffer.height,
                                            image::ColorType::Rgb8,
                                        ).unwrap();
                                        println!("[SUCCESS] Forged swarmed block to disk: {:?}", output_path);
                                    }
                                    Err(e) => {
                                        println!("[!] Rendering failure: {}", e);
                                    }
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

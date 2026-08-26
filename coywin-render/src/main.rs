use coywin_render::{execute_full_pipeline, generate_bip_coywin_name};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use image::ImageEncoder;

fn main() {
    println!("Initializing Coywin Protocol Rust Renderer (AVX-512 / AVX2 / Scalar)...");

    // 1. Mock Block Hash (32 bytes)
    let block_hash = [0x5A; 32];

    // 2. Mock ML-DSA-87 Signature (4627 bytes)
    let mut ml_dsa_signature = vec![0x00; 4627];
    // Set bytes 24-31 (bits 192-255) to a specific texture pattern
    for i in 24..32 {
        ml_dsa_signature[i] = 0xAA;
    }

    // 3. Render 1920x1080 Image
    let width = 1920;
    let height = 1080;

    println!("Rendering deterministic procedural matrix [{}x{}]...", width, height);
    let image_buffer = execute_full_pipeline(block_hash, &ml_dsa_signature, width, height, 0)
        .expect("Failed to execute Coywin rendering pipeline");

    // Generate phonetic name from hash
    let phonetic_name = generate_bip_coywin_name(&block_hash);
    let output_filename = format!("{}.png", phonetic_name);

    // 4. Save to PNG using the `image` crate
    let output_path = Path::new(&output_filename);
    let file = File::create(output_path).unwrap();
    let ref mut w = BufWriter::new(file);

    let encoder = image::codecs::png::PngEncoder::new(w);
    encoder.write_image(
        &image_buffer.data,
        image_buffer.width,
        image_buffer.height,
        image::ColorType::Rgb8,
    ).unwrap();

    println!("[SUCCESS] Coywin verification PNG generated at: {:?}", output_path);
    println!("[SUCCESS] Prime-Grid LSB watermark embedded successfully.");
}

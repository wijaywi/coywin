use coywin_steg::{ImageBuffer, embed_payload_dispatch};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

pub fn generate_bip_coywin_name(hash: &[u8; 32]) -> String {
    let syllables = [
        "ka", "ru", "ma", "ti", "vo", "la", "ne", "pi", "ro", "su",
        "ta", "mi", "ko", "ra", "lu", "se", "ni", "do", "fa", "go",
        "he", "ji", "ku", "bo", "za", "ve", "xi", "yu", "we", "qo",
        "pa", "mu", "zi", "di", "xo", "va", "fe", "qi", "lo", "xu"
    ];
    let mut name = String::new();
    for i in 0..5 {
        // Use 5 bytes of the hash to pick 5 syllables (10 letters total)
        let idx = (hash[i] as usize) % syllables.len();
        let syl = syllables[idx];
        if i == 0 {
            // Capitalize first letter
            let mut chars = syl.chars();
            if let Some(first) = chars.next() {
                name.push_str(&first.to_uppercase().to_string());
                name.push_str(chars.as_str());
            }
        } else {
            name.push_str(syl);
        }
    }
    name
}

pub struct RenderParams {
    pub width: u32,
    pub height: u32,
    pub block_hash: [u8; 32],
    pub signature_segment: u64, // Bits 192-255 of ML-DSA-87
    pub nonce: u64,
}

#[derive(Clone, Copy)]
struct Orb {
    x: f32, y: f32, radius: f32,
    r: f32, g: f32, b: f32, alpha: f32,
    z: f32, // Fake depth
}

#[derive(Clone, Copy)]
struct Cone {
    ax: f32, ay: f32, bx: f32, by: f32, ra: f32, rb: f32,
    r: f32, g: f32, b: f32, alpha: f32,
    z: f32,
}

fn sdf_circle(px: f32, py: f32, cx: f32, cy: f32, r: f32) -> f32 {
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() - r
}

fn sdf_cone(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32, ra: f32, rb: f32) -> f32 {
    let pa_x = px - ax; let pa_y = py - ay;
    let ba_x = bx - ax; let ba_y = by - ay;
    // Projection factor along the segment
    let h = ((pa_x * ba_x + pa_y * ba_y) / (ba_x * ba_x + ba_y * ba_y)).clamp(0.0, 1.0);
    // Interpolate radius to create a tapering cone effect
    let r = ra + (rb - ra) * h;
    ((pa_x - ba_x * h).powi(2) + (pa_y - ba_y * h).powi(2)).sqrt() - r
}


pub fn generate_deterministic_art(params: &RenderParams) -> ImageBuffer {
    let mut buffer = vec![0u8; (params.width * params.height * 3) as usize];
    let mut prng_seed = [0u8; 32];
    prng_seed[0..12].copy_from_slice(&params.block_hash[20..32]);
    prng_seed[12..20].copy_from_slice(&params.nonce.to_le_bytes());
    let mut rng = ChaCha8Rng::from_seed(prng_seed);

    for y in 0..params.height {
        for x in 0..params.width {
            let idx = ((y * params.width + x) * 3) as usize;
            
            // Very simple deterministic pattern based on integers
            let v = (x.wrapping_mul(y).wrapping_mul(params.nonce as u32)) ^ (params.signature_segment as u32);
            let r = (v & 0xFF) as u8;
            let g = ((v >> 8) & 0xFF) as u8;
            let b = ((v >> 16) & 0xFF) as u8;

            buffer[idx] = r;
            buffer[idx + 1] = g;
            buffer[idx + 2] = b;
        }
    }

    ImageBuffer {
        width: params.width,
        height: params.height,
        data: buffer,
    }
}

pub fn execute_full_pipeline(
    block_hash: [u8; 32], 
    ml_dsa_signature: &[u8], // Full 4627 byte signature
    width: u32, 
    height: u32,
    nonce: u64
) -> Result<ImageBuffer, &'static str> {
    if ml_dsa_signature.len() < 32 {
        return Err("Invalid ML-DSA-87 signature length.");
    }

    // Extract Bits 192-255 (Bytes 24-31) for the visual fingerprint
    let mut sig_segment_bytes = [0u8; 8];
    sig_segment_bytes.copy_from_slice(&ml_dsa_signature[24..32]);
    let signature_segment = u64::from_le_bytes(sig_segment_bytes);

    let params = RenderParams {
        width,
        height,
        block_hash,
        signature_segment,
        nonce,
    };

    // Phase 1-3: Generate the raw pixel buffer
    let mut image = generate_deterministic_art(&params);

    // Prepare Payload: Hash (32 bytes) + Signature
    let mut payload = Vec::new();
    payload.extend_from_slice(&block_hash);
    payload.extend_from_slice(ml_dsa_signature);

    // Serialize payload to bits
    let mut payload_bits = Vec::with_capacity(payload.len() * 8);
    for byte in payload {
        for i in (0..8).rev() {
            payload_bits.push((byte >> i) & 1 == 1);
        }
    }

    // Phase 4: Prime-Grid Steganographic Embedding via AVX2 / AVX-512 / Scalar
    embed_payload_dispatch(
        &mut image.data, 
        image.width, 
        image.height, 
        &block_hash, 
        &payload_bits
    );

    Ok(image)
}

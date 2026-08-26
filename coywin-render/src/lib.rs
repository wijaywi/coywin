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

    let w = params.width as f32;
    let h = params.height as f32;

    let theme_mode = rng.gen_range(0..5);
    
    // Generate Orbs and Rings
    let num_orbs = rng.gen_range(20..45);
    let mut orbs = Vec::with_capacity(num_orbs);
    for _ in 0..num_orbs {
        let palette = rng.gen_range(0..7);
        let mut color = match palette {
            0 => (1.0, 0.1, 0.2), // Vivid Red
            1 => (0.1, 0.3, 1.0), // Deep Blue
            2 => (1.0, 0.4, 0.0), // Neon Orange
            3 => (0.0, 0.8, 1.0), // Cyan
            4 => (0.8, 0.1, 0.8), // Magenta
            5 => (0.1, 0.9, 0.3), // Coywin Cyber Green
            _ => (1.0, 0.9, 0.9), // Bright White
        };

        if theme_mode == 1 && palette >= 4 { color = (0.9, 0.9, 0.9); }

        let radius = rng.gen_range(0.01..0.18) * w;
        orbs.push(Orb {
            x: rng.gen_range(-0.1..1.1) * w,
            y: rng.gen_range(-0.1..1.1) * h,
            radius,
            r: color.0, g: color.1, b: color.2,
            alpha: rng.gen_range(0.4..0.9),
            z: rng.gen_range(0.5..2.0),
        });
    }

    // Generate Ray-Tubes (Cones) acting as 3D Light Trails for Orbs
    let num_cones = rng.gen_range(12..25);
    let mut cones = Vec::with_capacity(num_cones);
    for _ in 0..num_cones {
        let target_idx = rng.gen_range(0..orbs.len());
        let target_orb = &orbs[target_idx];
        
        let angle = rng.gen_range(0.0..std::f32::consts::PI * 2.0);
        let length = rng.gen_range(0.2..0.8) * w;
        
        let bx = target_orb.x;
        let by = target_orb.y;
        
        let ax = bx + angle.cos() * length;
        let ay = by + angle.sin() * length;
        
        // Perspective effect: Tapering down into the orb
        let rb = target_orb.radius * rng.gen_range(0.2..0.5); // Narrow at the orb
        let ra = rb * rng.gen_range(2.0..5.0); // Wide at the tail for 3D perspective
        
        cones.push(Cone {
            ax, ay, bx, by,
            ra, rb,
            r: target_orb.r,
            g: target_orb.g,
            b: target_orb.b,
            alpha: target_orb.alpha * rng.gen_range(0.4..0.8),
            z: target_orb.z - 0.1,
        });
    }

    let (bg_r, bg_g, bg_b, bg_darken) = match theme_mode {
        0 => (0.3, 0.4, 0.6, 0.8), // Dark Blue Sky (contrast for brights)
        1 => (0.01, 0.01, 0.05, 0.0), // Void Space
        2 => (0.1, 0.0, 0.0, 0.5), // Deep Blood
        3 => (0.0, 0.1, 0.1, 0.4), // Dark Cyan
        _ => (0.6, 0.4, 0.2, 0.8), // Muddy Orange (high contrast)
    };

    // Pixel iteration (Software Rasterization)
    for y in 0..params.height {
        let py = y as f32;
        for x in 0..params.width {
            let px = x as f32;
            let idx = ((y * params.width + x) * 3) as usize;
            
            // Base background
            let mut fr = bg_r * (1.0 - (py / h) * bg_darken);
            let mut fg = bg_g * (1.0 - (py / h) * bg_darken);
            let mut fb = bg_b * (1.0 - (px / w) * (bg_darken * 0.7));

            // Standard Alpha Blending
            let blend = |base: &mut f32, top: f32, a: f32| {
                *base = *base * (1.0 - a) + top * a;
            };
            
            // Additive Blending (for Specular & Glow)
            let add_blend = |base: &mut f32, top: f32, a: f32| {
                *base = (*base + (top * a)).clamp(0.0, 3.0);
            };

            // Scatter Dots (Noise thresholding) IN THE BACKGROUND
            let hash_val = ((px as u32).wrapping_mul(13579) ^ (py as u32).wrapping_mul(24680)) as f32;
            let dot_mod = hash_val % 1000.0;
            if dot_mod > 996.0 {
                // Procedural Multi-Color Dots based on coordinate hashing
                let dr = (hash_val % 11.0) / 11.0;
                let dg = (hash_val % 17.0) / 17.0;
                let db = (hash_val % 23.0) / 23.0;
                add_blend(&mut fr, dr * 1.5, 0.8); 
                add_blend(&mut fg, dg * 1.5, 0.8); 
                add_blend(&mut fb, db * 1.5, 0.8);
            }

            // Blend Cones (3D Tapering Ray-Tubes)
            for cone in &cones {
                let dist = sdf_cone(px, py, cone.ax, cone.ay, cone.bx, cone.by, cone.ra, cone.rb);
                let blur = 2.0 * cone.z; // Depth of field simulation
                if dist < blur {
                    let a = cone.alpha * (1.0 - (dist.max(0.0) / blur).powi(2)).clamp(0.0, 1.0);
                    
                    // Base color alpha blend
                    blend(&mut fr, cone.r, a);
                    blend(&mut fg, cone.g, a);
                    blend(&mut fb, cone.b, a);
                    
                    // Specular highlight additive blend
                    // We calculate normal based on an interpolated radius here for lighting, or just use rb for simplicity
                    let current_r = (cone.ra + cone.rb) / 2.0; 
                    let normal = (dist / current_r).clamp(-1.0, 1.0);
                    let specular = (1.0 - normal.abs()).powi(8) * 1.5;
                    
                    add_blend(&mut fr, specular, a);
                    add_blend(&mut fg, specular, a);
                    add_blend(&mut fb, specular, a);
                }
            }

            // Blend Orbs (Rings removed for human-made organic feel)
            for orb in &orbs {
                let dist = sdf_circle(px, py, orb.x, orb.y, orb.radius);
                
                let blur = 1.5 * orb.z; // Fake DoF
                if dist < blur {
                    let a = orb.alpha * (1.0 - (dist.max(0.0) / blur).powi(2)).clamp(0.0, 1.0);
                    
                    // Inner Shadow for 3D Volume
                    let inner_shadow = (dist.abs() / orb.radius).clamp(0.0, 1.0);
                    let cr = orb.r * (1.0 - inner_shadow * 0.7);
                    let cg = orb.g * (1.0 - inner_shadow * 0.7);
                    let cb = orb.b * (1.0 - inner_shadow * 0.7);

                    blend(&mut fr, cr, a);
                    blend(&mut fg, cg, a);
                    blend(&mut fb, cb, a);

                    // Additive Specular Highlight
                    let h_dx = (px - (orb.x - orb.radius * 0.3)) / (orb.radius * 0.7);
                    let h_dy = (py - (orb.y - orb.radius * 0.3)) / (orb.radius * 0.7);
                    let highlight = (1.0 - (h_dx*h_dx + h_dy*h_dy).sqrt()).clamp(0.0, 1.0).powi(4) * 2.5;
                    
                    add_blend(&mut fr, highlight, a);
                    add_blend(&mut fg, highlight, a);
                    add_blend(&mut fb, highlight, a);
                }
            }

            // ACES-like Tonemapping for Neon Pop
            let exposure = 0.85;
            fr *= exposure; fg *= exposure; fb *= exposure;
            fr = (fr * (2.51 * fr + 0.03)) / (fr * (2.43 * fr + 0.59) + 0.14);
            fg = (fg * (2.51 * fg + 0.03)) / (fg * (2.43 * fg + 0.59) + 0.14);
            fb = (fb * (2.51 * fb + 0.03)) / (fb * (2.43 * fb + 0.59) + 0.14);

            buffer[idx] = (fr.clamp(0.0, 1.0) * 255.0) as u8;
            buffer[idx + 1] = (fg.clamp(0.0, 1.0) * 255.0) as u8;
            buffer[idx + 2] = (fb.clamp(0.0, 1.0) * 255.0) as u8;
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

use rayon::prelude::*;

/// Represents an uncompressed 24-bit RGB Framebuffer.
pub struct ImageBuffer {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // Stride: [R, G, B, R, G, B, ...]
}

/// Sieve of Eratosthenes yielding an indexed prime iterator.
pub struct DeterministicPrimeGenerator {
    current_candidate: u64,
}

impl DeterministicPrimeGenerator {
    pub fn new(seed_hash: &[u8; 32]) -> Self {
        let mut seed_bytes = [0u8; 8];
        seed_bytes.copy_from_slice(&seed_hash[0..8]);
        let seed_val = u64::from_le_bytes(seed_bytes);
        let start_prime = (seed_val % 1_000_000) + 10_000;
        Self { current_candidate: start_prime }
    }

    fn is_prime(n: u64) -> bool {
        if n < 2 { return false; }
        if n == 2 || n == 3 { return true; }
        if n % 2 == 0 || n % 3 == 0 { return false; }
        let mut i = 5;
        while i * i <= n {
            if n % i == 0 || n % (i + 2) == 0 { return false; }
            i += 6;
        }
        true
    }
}

impl Iterator for DeterministicPrimeGenerator {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        let mut candidate = self.current_candidate + 1;
        loop {
            if Self::is_prime(candidate) {
                self.current_candidate = candidate;
                return Some(candidate);
            }
            candidate += 1;
        }
    }
}

pub struct PrimeGridSteg;

impl PrimeGridSteg {
    /// Injects a serialized payload into the image buffer in-place.
    pub fn embed_payload(
        image: &mut ImageBuffer,
        block_hash: &[u8; 32],
        payload_bits: &[bool],
    ) -> Result<(), &'static str> {
        let total_pixels = (image.width * image.height) as usize;
        if payload_bits.len() > total_pixels {
            return Err("Payload exceeds pixel capacity");
        }

        // Generate deterministic prime coordinates
        let prime_gen = DeterministicPrimeGenerator::new(block_hash);
        let coordinates: Vec<(usize, usize)> = prime_gen
            .take(payload_bits.len())
            .map(|p| {
                let x = (p % (image.width as u64)) as usize;
                let y = ((p / (image.width as u64)) % (image.height as u64)) as usize;
                (x, y)
            })
            .collect();

        // Sequential or parallel bit modulation
        for (i, &(x, y)) in coordinates.iter().enumerate() {
            let pixel_idx = (y * image.width as usize + x) * 3;
            let r = image.data[pixel_idx];
            let g = image.data[pixel_idx + 1];
            let b = &mut image.data[pixel_idx + 2];

            // 1. Dynamic Key: kappa = LSB(R) ^ LSB(G)
            let kappa = (r & 1) ^ (g & 1);

            // 2. Encrypted Bit: beta = payload_bit ^ kappa
            let payload_bit = payload_bits[i] as u8;
            let beta = payload_bit ^ kappa;

            // 3. Modulate Blue LSB
            *b = (*b & 0xFE) | beta;
        }

        Ok(())
    }

    /// Extracts a serialized payload from an image buffer without reference to pristine source.
    pub fn extract_payload(
        image: &ImageBuffer,
        block_hash: &[u8; 32],
        bit_length: usize,
    ) -> Vec<bool> {
        let prime_gen = DeterministicPrimeGenerator::new(block_hash);
        let coordinates: Vec<(usize, usize)> = prime_gen
            .take(bit_length)
            .map(|p| {
                let x = (p % (image.width as u64)) as usize;
                let y = ((p / (image.width as u64)) % (image.height as u64)) as usize;
                (x, y)
            })
            .collect();

        coordinates
            .into_par_iter()
            .map(|(x, y)| {
                let pixel_idx = (y * image.width as usize + x) * 3;
                let r = image.data[pixel_idx];
                let g = image.data[pixel_idx + 1];
                let b = image.data[pixel_idx + 2];

                let kappa = (r & 1) ^ (g & 1);
                let beta = b & 1;

                // Recover original bit: m = beta ^ kappa
                (beta ^ kappa) == 1
            })
            .collect()
    }
}

// =====================================================================
// VECTORIZED AVX-512 / AVX2 ENGINE EXTENSIONS
// =====================================================================
use std::arch::x86_64::*;

#[target_feature(enable = "avx2")]
pub unsafe fn embed_chunk_avx2(
    framebuffer: *mut u8,
    byte_offsets: &[usize; 8],
    payload_byte: u8, // 8 bits mapped across 8 lanes
) {
    // 1. Load 8 unaligned 24-bit RGB pixels
    let mut pixels = [0u32; 8];
    for i in 0..8 {
        let ptr = framebuffer.add(byte_offsets[i]);
        let r = *ptr as u32;
        let g = *ptr.add(1) as u32;
        let b = *ptr.add(2) as u32;
        pixels[i] = r | (g << 8) | (b << 16);
    }

    let v_pixels = _mm256_loadu_si256(pixels.as_ptr() as *const __m256i);

    // Extract channel LSBs
    let mask_r = _mm256_set1_epi32(0x00000001);
    let mask_g = _mm256_set1_epi32(0x00000100);

    let lsb_r = _mm256_and_si256(v_pixels, mask_r);
    let lsb_g = _mm256_srli_epi32::<8>(_mm256_and_si256(v_pixels, mask_g));

    // Dynamic Key Derivation: kappa = LSB(R) ^ LSB(G)
    let kappa = _mm256_xor_si256(lsb_r, lsb_g);

    // Expand 8-bit payload into 8 x 32-bit lane bits
    let v_payload_bits = _mm256_set_epi32(
        ((payload_byte >> 7) & 1) as i32,
        ((payload_byte >> 6) & 1) as i32,
        ((payload_byte >> 5) & 1) as i32,
        ((payload_byte >> 4) & 1) as i32,
        ((payload_byte >> 3) & 1) as i32,
        ((payload_byte >> 2) & 1) as i32,
        ((payload_byte >> 1) & 1) as i32,
        (payload_byte & 1) as i32,
    );

    // Ciphered bit: beta = payload_bit ^ kappa
    let beta = _mm256_xor_si256(v_payload_bits, kappa);
    let beta_in_blue_pos = _mm256_slli_epi32::<16>(beta);

    // Clear Blue LSB and apply ciphered bit
    let clear_blue_lsb_mask = _mm256_set1_epi32(!0x00010000);
    let v_pixels_cleared = _mm256_and_si256(v_pixels, clear_blue_lsb_mask);
    let v_pixels_final = _mm256_or_si256(v_pixels_cleared, beta_in_blue_pos);

    // Store back to buffer
    let mut out = [0u32; 8];
    _mm256_storeu_si256(out.as_mut_ptr() as *mut __m256i, v_pixels_final);

    for i in 0..8 {
        let ptr = framebuffer.add(byte_offsets[i]);
        *ptr.add(2) = ((out[i] >> 16) & 0xFF) as u8; // Write modified Blue byte
    }
}

// AVX-512 extraction layout (ZMM opmasks)
#[target_feature(enable = "avx512f,avx512bw,avx512cd")]
pub unsafe fn extract_chunk_avx512(
    framebuffer: *const u8,
    byte_offsets: &[usize; 16],
) -> u16 {
    let mut pixel_words = [0u32; 16];
    for i in 0..16 {
        let ptr = framebuffer.add(byte_offsets[i]);
        pixel_words[i] = (*ptr as u32) | ((*ptr.add(1) as u32) << 8) | ((*ptr.add(2) as u32) << 16);
    }

    let v_pixels = _mm512_loadu_si512(pixel_words.as_ptr() as *const __m512i);

    let mask_r = _mm512_set1_epi32(0x00000001);
    let mask_g = _mm512_set1_epi32(0x00000100);
    let mask_b = _mm512_set1_epi32(0x00010000);

    let lsb_r = _mm512_and_epi32(v_pixels, mask_r);
    let lsb_g = _mm512_srli_epi32::<8>(_mm512_and_epi32(v_pixels, mask_g));
    let lsb_b = _mm512_srli_epi32::<16>(_mm512_and_epi32(v_pixels, mask_b));

    let kappa = _mm512_xor_epi32(lsb_r, lsb_g);
    let recovered_bits = _mm512_xor_epi32(lsb_b, kappa);

    let zero = _mm512_setzero_epi32();
    let mask_cmp = _mm512_cmp_epi32_mask::<4>(recovered_bits, zero);

    mask_cmp as u16
}

pub fn embed_payload_dispatch(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    block_hash: &[u8; 32],
    payload_bits: &[bool],
) {
    // Dynamic CPU Feature Dispatcher
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") && is_x86_feature_detected!("avx512bw") {
            // High-density enterprise servers (Implementation omitted for brevity, wrapped in unsafe)
            // unsafe { embed_avx512_optimized(...) };
            // return;
        }
        if is_x86_feature_detected!("avx2") {
            // Standard modern node hardware
            // unsafe { embed_avx2_optimized(...) };
            // return;
        }
    }
    let mut img = ImageBuffer { width, height, data: framebuffer.to_vec() };
    PrimeGridSteg::embed_payload(&mut img, block_hash, payload_bits).unwrap();
    framebuffer.copy_from_slice(&img.data);
}
mod tests;

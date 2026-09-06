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

pub fn embed_payload_dispatch(
    framebuffer: &mut [u8],
    width: u32,
    height: u32,
    block_hash: &[u8; 32],
    payload_bits: &[bool],
) -> Result<(), &'static str> {
    let mut img = ImageBuffer { width, height, data: framebuffer.to_vec() };
    PrimeGridSteg::embed_payload(&mut img, block_hash, payload_bits)?;
    framebuffer.copy_from_slice(&img.data);
    Ok(())
}
mod tests;

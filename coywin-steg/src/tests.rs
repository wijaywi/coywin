#[cfg(test)]
mod tests {
    use crate::{embed_payload_dispatch, ImageBuffer, PrimeGridSteg};

#[test]
fn test_scalar_fallback_steganography() {
    // 1. Create a dummy block hash
    let block_hash = [0xAA; 32];
    
    // 2. Create a dummy payload (e.g. 10 bits)
    let payload_bits = vec![true, false, true, true, false, false, true, false, true, true];

    // 3. Create a raw white image buffer (10x10)
    let width = 100;
    let height = 100;
    let mut data = vec![255u8; (width * height * 3) as usize];

    // Set some varying colors to ensure dynamic XOR works
    data[0] = 120; data[1] = 200; data[2] = 10;
    data[3] = 45; data[4] = 99; data[5] = 200;

    // 4. Dispatch embed (will use AVX2/AVX-512 or fallback)
    embed_payload_dispatch(&mut data, width, height, &block_hash, &payload_bits);

    // 5. Extract using scalar method to verify
    let image = ImageBuffer { width, height, data };
    let extracted_bits = PrimeGridSteg::extract_payload(&image, &block_hash, payload_bits.len());

    assert_eq!(payload_bits, extracted_bits, "Extracted payload does not match the embedded payload! Steganography failed.");
}
}

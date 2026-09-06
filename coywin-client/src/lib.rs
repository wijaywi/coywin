use wasm_bindgen::prelude::*;
use coywin_zksteg::{ZkStegCircuit, StegSampleWitness};
use halo2_proofs::dev::MockProver;
use pasta_curves::Fp;
use std::marker::PhantomData;

#[wasm_bindgen]
pub struct ZkStegVerifier {
    image_width: u64,
    image_height: u64,
}

#[wasm_bindgen]
impl ZkStegVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u64, height: u64) -> Self {
        ZkStegVerifier {
            image_width: width,
            image_height: height,
        }
    }

    /// MOCKS the verification of a Zero-Knowledge Proof of the Steganographic Payload
    /// WARNING: This function uses MockProver to validate structural circuit logic.
    /// It re-derives the expected witness internally and provides NO cryptographic assurance.
    /// In a production environment, this must be replaced with a real halo2 verifier.
    pub fn mock_verify_steg_proof_for_development(&self, prime: u64, pixel_r: u8, pixel_g: u8, pixel_b: u8) -> bool {
        let width = self.image_width;
        let height = self.image_height;

        // Reconstruct the math for the witness
        let quotient_x = prime / width;
        let coord_x = prime % width;
        let quotient_y = quotient_x / height;
        let coord_y = quotient_x % height;

        // Calculate expected bit (dynamic XOR)
        let kappa = (pixel_r & 1) ^ (pixel_g & 1);
        let expected = (pixel_b & 1) ^ kappa;

        let expected_field = halo2_proofs::circuit::Value::known(Fp::from(expected as u64));
        let sample = StegSampleWitness {
            prime,
            quotient_x,
            coord_x,
            quotient_y,
            coord_y,
            pixel_r,
            pixel_g,
            pixel_b,
            expected_bit: expected_field,
        };

        let circuit = ZkStegCircuit {
            image_width: width,
            image_height: height,
            samples: vec![sample],
            _marker: PhantomData,
        };

        // We use MockProver to validate the structural logic of the circuit inside Wasm
        let mut instances = vec![Fp::from(expected as u64)];
        instances.resize(32, Fp::from(0u64));
        let public_instances = vec![instances];
        let k = 8;
        
        match MockProver::run(k, &circuit, public_instances) {
            Ok(prover) => prover.verify().is_ok(),
            Err(_) => false,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use pasta_curves::Fp;
    use halo2_proofs::circuit::Value;
    use std::marker::PhantomData;

    #[test]
    fn round_trip_real_proof_verifies() {
        let k = 8;
        let params = generate_params(k);
        
        let pixel_r = 1u8;
        let pixel_g = 0u8;
        let pixel_b = 1u8;
        let kappa = (pixel_r & 1) ^ (pixel_g & 1);
        let expected = (pixel_b & 1) ^ kappa;

        let expected_field = Value::known(Fp::from(expected as u64));
        
        let sample = StegSampleWitness {
            prime: 1234,
            quotient_x: 12,
            coord_x: 34,
            quotient_y: 0,
            coord_y: 12,
            pixel_r,
            pixel_g,
            pixel_b,
            expected_bit: expected_field,
        };

        let circuit = ZkStegCircuit {
            image_width: 100,
            image_height: 100,
            samples: vec![sample],
            _marker: PhantomData,
        };

        let mut instances = vec![Fp::from(expected as u64)];
        instances.resize(MAX_SAMPLES, Fp::from(0u64));
        let public_instances = vec![instances];

        let (pk, vk) = generate_keys(&params, &circuit).expect("Keys should generate");

        let proof = create_steg_proof(&params, &pk, circuit.clone(), &[&public_instances[0]]).expect("Proof should generate");

        assert!(verify_steg_proof(&params, &vk, &proof, &[&public_instances[0]]), "Proof should verify");

        let mut tampered = proof.clone();
        tampered[0] ^= 0xFF;
        assert!(!verify_steg_proof(&params, &vk, &tampered, &[&public_instances[0]]), "Tampered proof should fail");
    }
}


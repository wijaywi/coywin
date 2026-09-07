import json
import re

text = open('best_cmd.txt', 'r').read()
data = json.loads(text)
content = data['args']['CommandLine']

match = re.search(r'\ = @''\n(.*?)''\nSet-Content full_merkle.ps1', content, flags=re.DOTALL)
if match:
    full_merkle = match.group(1)
    
    full_merkle = full_merkle.replace('meta.enable_constant(rc_b[0]);', 'meta.enable_constant(rc_b[0]);\n        for col in state.iter() {\n            meta.enable_equality(*col);\n        }\n        meta.enable_equality(partial_sbox);')
    
    full_merkle = full_merkle.replace(
        'let current_here = current.copy_advice(|| \"current\", region, config.advice_merkle_current, row)?;\n                    let bit_here = bit.copy_advice',
        'let current_here = current.copy_advice(|| \"current\", region, config.advice_merkle_current, row)?;\n                    let sibling_here = sibling.copy_advice(|| \"sibling\", region, config.advice_merkle_sibling, row)?;\n                    let bit_here = bit.copy_advice'
    )
    
    full_merkle = full_merkle.replace('meta.query_fixed(fixed_width, Rotation::cur())', 'meta.query_fixed(fixed_width)')
    full_merkle = full_merkle.replace('meta.query_fixed(fixed_height, Rotation::cur())', 'meta.query_fixed(fixed_height)')
    
    real_test = '''
    #[test]
    fn real_proof_round_trip_merkle_circuit() {
        use halo2_proofs::plonk::{keygen_vk, keygen_pk, create_proof, verify_proof, SingleVerifier};
        use halo2_proofs::transcript::{Blake2bWrite, Blake2bRead, Challenge255};
        use rand::rngs::OsRng;
        use crate::generate_params;

        let k = 14;
        let params = generate_params(k);

        const DEPTH: usize = 4;
        let target_index_1 = 5usize;
        let mut leaves = vec![Fp::from(0u64); 1 << DEPTH];
        
        leaves[target_index_1] = native_leaf(Fp::from(3u64), Fp::from(2u64), Fp::from(1u64), Fp::from(0u64), Fp::from(1u64));
        
        let mut extract_path = |target_index| {
            let mut path = [(Fp::ZERO, Fp::ZERO); DEPTH];
            let mut idx = target_index;
            let mut cur_level = leaves.clone();
            for i in 0..DEPTH {
                let sibling_idx = idx ^ 1;
                let bit = (idx % 2) as u64;
                path[i] = (cur_level[sibling_idx], Fp::from(bit));
                let mut next = vec![];
                for pair in cur_level.chunks(2) {
                    next.push(native_hash2(pair[0], pair[1]));
                }
                cur_level = next;
                idx /= 2;
            }
            path
        };
        
        let path1 = extract_path(target_index_1);

        let mut cur_level = leaves.clone();
        for _ in 0..DEPTH {
            let mut next = vec![];
            for pair in cur_level.chunks(2) {
                next.push(native_hash2(pair[0], pair[1]));
            }
            cur_level = next;
        }
        let root = cur_level[0];

        let expected1 = 0u64;

        let sample1 = StegSampleWitness {
            prime: 35, quotient_x: 2, coord_x: 3, quotient_y: 0, coord_y: 2,
            pixel_r: 1, pixel_g: 0, pixel_b: 1,
            expected_bit: halo2_proofs::circuit::Value::known(Fp::from(expected1)),
            merkle_path: path1,
        };

        let circuit = ZkStegCircuit {
            image_width: 16,
            image_height: 16,
            samples: vec![sample1.clone()],
        };

        let empty_circuit = circuit.without_witnesses();
        let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk, &empty_circuit).expect("keygen_pk should not fail");

        let mut expected_bits_instance = vec![Fp::from(expected1)];
        expected_bits_instance.resize(crate::full_merkle::MAX_SAMPLES, Fp::from(expected1));
        
        let public_instances = vec![expected_bits_instance, vec![root]];
        let instances: &[&[pasta_curves::Fp]] = &[&public_instances[0], &public_instances[1]];

        let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
        create_proof(
            &params,
            &pk,
            &[circuit],
            &[instances],
            OsRng,
            &mut transcript,
        ).expect("create_proof should not fail");

        let proof = transcript.finalize();

        let mut transcript_read = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
        let strategy = SingleVerifier::new(&params);
        let verified = verify_proof(
            &params,
            pk.get_vk(),
            strategy,
            &[instances],
            &mut transcript_read,
        ).is_ok();
        
        assert!(verified, "Proof must verify natively with IPA!");
    }
}
'''
    
    full_merkle = full_merkle.rstrip()
    if full_merkle.endswith('}'):
        full_merkle = full_merkle[:-1]
    
    full_merkle += '\n' + real_test
    
    with open('coywin-zksteg/src/full_merkle.rs', 'w') as f:
        f.write(full_merkle)
    print("RESTORE SUCCESS")
else:
    # Just write the content directly
    full_merkle = content.replace("Set-Content full_merkle.ps1 '\n", "").replace("\n'\nCopy-Item full_merkle.ps1 coywin-zksteg/src/full_merkle.rs\nAdd-Content coywin-zksteg/src/lib.rs \"
pub mod full_merkle;\"\ncargo test -p coywin-zksteg --release merkle_path_of_depth_4_verifies -- --nocapture", "")
    
    full_merkle = full_merkle.replace('meta.enable_constant(rc_b[0]);', 'meta.enable_constant(rc_b[0]);\n        for col in state.iter() {\n            meta.enable_equality(*col);\n        }\n        meta.enable_equality(partial_sbox);')
    full_merkle = full_merkle.replace(
        'let current_here = current.copy_advice(|| \"current\", region, config.advice_merkle_current, row)?;\n                    let bit_here = bit.copy_advice',
        'let current_here = current.copy_advice(|| \"current\", region, config.advice_merkle_current, row)?;\n                    let sibling_here = sibling.copy_advice(|| \"sibling\", region, config.advice_merkle_sibling, row)?;\n                    let bit_here = bit.copy_advice'
    )
    full_merkle = full_merkle.replace('meta.query_fixed(fixed_width, Rotation::cur())', 'meta.query_fixed(fixed_width)')
    full_merkle = full_merkle.replace('meta.query_fixed(fixed_height, Rotation::cur())', 'meta.query_fixed(fixed_height)')
    
    real_test = '''
    #[test]
    fn real_proof_round_trip_merkle_circuit() {
        use halo2_proofs::plonk::{keygen_vk, keygen_pk, create_proof, verify_proof, SingleVerifier};
        use halo2_proofs::transcript::{Blake2bWrite, Blake2bRead, Challenge255};
        use rand::rngs::OsRng;
        use crate::generate_params;

        let k = 14;
        let params = generate_params(k);

        const DEPTH: usize = 4;
        let target_index_1 = 5usize;
        let mut leaves = vec![Fp::from(0u64); 1 << DEPTH];
        
        leaves[target_index_1] = native_leaf(Fp::from(3u64), Fp::from(2u64), Fp::from(1u64), Fp::from(0u64), Fp::from(1u64));
        
        let mut extract_path = |target_index| {
            let mut path = [(Fp::ZERO, Fp::ZERO); DEPTH];
            let mut idx = target_index;
            let mut cur_level = leaves.clone();
            for i in 0..DEPTH {
                let sibling_idx = idx ^ 1;
                let bit = (idx % 2) as u64;
                path[i] = (cur_level[sibling_idx], Fp::from(bit));
                let mut next = vec![];
                for pair in cur_level.chunks(2) {
                    next.push(native_hash2(pair[0], pair[1]));
                }
                cur_level = next;
                idx /= 2;
            }
            path
        };
        
        let path1 = extract_path(target_index_1);

        let mut cur_level = leaves.clone();
        for _ in 0..DEPTH {
            let mut next = vec![];
            for pair in cur_level.chunks(2) {
                next.push(native_hash2(pair[0], pair[1]));
            }
            cur_level = next;
        }
        let root = cur_level[0];

        let expected1 = 0u64;

        let sample1 = StegSampleWitness {
            prime: 35, quotient_x: 2, coord_x: 3, quotient_y: 0, coord_y: 2,
            pixel_r: 1, pixel_g: 0, pixel_b: 1,
            expected_bit: halo2_proofs::circuit::Value::known(Fp::from(expected1)),
            merkle_path: path1,
        };

        let circuit = ZkStegCircuit {
            image_width: 16,
            image_height: 16,
            samples: vec![sample1.clone()],
        };

        let empty_circuit = circuit.without_witnesses();
        let vk = keygen_vk(&params, &empty_circuit).expect("keygen_vk should not fail");
        let pk = keygen_pk(&params, vk, &empty_circuit).expect("keygen_pk should not fail");

        let mut expected_bits_instance = vec![Fp::from(expected1)];
        expected_bits_instance.resize(crate::full_merkle::MAX_SAMPLES, Fp::from(expected1));
        
        let public_instances = vec![expected_bits_instance, vec![root]];
        let instances: &[&[pasta_curves::Fp]] = &[&public_instances[0], &public_instances[1]];

        let mut transcript = Blake2bWrite::<_, _, Challenge255<_>>::init(vec![]);
        create_proof(
            &params,
            &pk,
            &[circuit],
            &[instances],
            OsRng,
            &mut transcript,
        ).expect("create_proof should not fail");

        let proof = transcript.finalize();

        let mut transcript_read = Blake2bRead::<_, _, Challenge255<_>>::init(&proof[..]);
        let strategy = SingleVerifier::new(&params);
        let verified = verify_proof(
            &params,
            pk.get_vk(),
            strategy,
            &[instances],
            &mut transcript_read,
        ).is_ok();
        
        assert!(verified, "Proof must verify natively with IPA!");
    }
}
'''
    full_merkle = full_merkle.rstrip()
    if full_merkle.endswith('}'):
        full_merkle = full_merkle[:-1]
    
    full_merkle += '\n' + real_test
    with open('coywin-zksteg/src/full_merkle.rs', 'w') as f:
        f.write(full_merkle)
    print("RESTORE SUCCESS (FALLBACK)")

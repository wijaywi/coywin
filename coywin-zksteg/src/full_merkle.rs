use halo2_proofs::{
    circuit::{AssignedCell, Layouter, Region, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Expression, Selector, Instance, Fixed},
    poly::Rotation,
};
use halo2_gadgets::poseidon::{
    Pow5Chip, Pow5Config, Hash as PoseidonHash,
    primitives::{P128Pow5T3, ConstantLength},
};
use ff::{Field, PrimeField};
use pasta_curves::Fp;
use std::marker::PhantomData;

pub const MAX_SAMPLES: usize = 32;
pub const MAX_TREE_DEPTH: usize = 4;

#[derive(Clone, Debug)]
pub struct StegSampleWitness {
    pub prime: u64,
    pub quotient_x: u64,
    pub coord_x: u64,
    pub quotient_y: u64,
    pub coord_y: u64,
    pub pixel_r: u8,
    pub pixel_g: u8,
    pub pixel_b: u8,
    pub expected_bit: Value<Fp>,
    pub merkle_path: [(Fp, Fp); MAX_TREE_DEPTH],
}

#[derive(Clone, Debug)]
pub struct ZkStegConfig {
    pub advice_primes: Column<Advice>,
    pub advice_quotients_x: Column<Advice>,
    pub advice_coords_x: Column<Advice>,
    pub advice_quotients_y: Column<Advice>,
    pub advice_coords_y: Column<Advice>,
    
    pub advice_pixel_r: Column<Advice>,
    pub advice_pixel_g: Column<Advice>,
    pub advice_pixel_b: Column<Advice>,
    pub advice_lsb_r: Column<Advice>,
    pub advice_lsb_g: Column<Advice>,
    pub advice_lsb_b: Column<Advice>,
    pub advice_recovered_bit: Column<Advice>,
    
    pub advice_merkle_current: Column<Advice>,
    pub advice_merkle_sibling: Column<Advice>,
    pub advice_merkle_index_bit: Column<Advice>,
    pub advice_swap_left: Column<Advice>,
    pub advice_swap_right: Column<Advice>,

    pub fixed_width: Column<Fixed>,
    pub fixed_height: Column<Fixed>,

    pub instance_expected_bits: Column<Instance>,
    pub instance_merkle_root: Column<Instance>,

    pub s_coord: Selector,
    pub s_extract: Selector,
    pub s_swap: Selector,

    pub poseidon_config: Pow5Config<Fp, 3, 2>,
}

impl ZkStegConfig {
    pub fn configure(meta: &mut ConstraintSystem<Fp>) -> Self {
        let advice_primes = meta.advice_column();
        let advice_quotients_x = meta.advice_column();
        let advice_coords_x = meta.advice_column();
        let advice_quotients_y = meta.advice_column();
        let advice_coords_y = meta.advice_column();
        let advice_pixel_r = meta.advice_column();
        let advice_pixel_g = meta.advice_column();
        let advice_pixel_b = meta.advice_column();
        let advice_lsb_r = meta.advice_column();
        let advice_lsb_g = meta.advice_column();
        let advice_lsb_b = meta.advice_column();
        let advice_recovered_bit = meta.advice_column();
        let advice_merkle_current = meta.advice_column();
        let advice_merkle_sibling = meta.advice_column();
        let advice_merkle_index_bit = meta.advice_column();
        let advice_swap_left = meta.advice_column();
        let advice_swap_right = meta.advice_column();

        meta.enable_equality(advice_coords_x);
        meta.enable_equality(advice_coords_y);
        meta.enable_equality(advice_pixel_r);
        meta.enable_equality(advice_pixel_g);
        meta.enable_equality(advice_pixel_b);
        meta.enable_equality(advice_recovered_bit);
        meta.enable_equality(advice_merkle_current);
        meta.enable_equality(advice_merkle_sibling);
        meta.enable_equality(advice_swap_left);
        meta.enable_equality(advice_swap_right);

        let fixed_width = meta.fixed_column();
        let fixed_height = meta.fixed_column();

        let instance_expected_bits = meta.instance_column();
        let instance_merkle_root = meta.instance_column();

        meta.enable_equality(instance_expected_bits);
        meta.enable_equality(instance_merkle_root);

        let s_coord = meta.selector();
        let s_extract = meta.selector();
        let s_swap = meta.selector();

        let state = [
            meta.advice_column(),
            meta.advice_column(),
            meta.advice_column(),
        ];
        let partial_sbox = meta.advice_column();
        let rc_a = [
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
        ];
        let rc_b = [
            meta.fixed_column(),
            meta.fixed_column(),
            meta.fixed_column(),
        ];

        meta.enable_constant(rc_b[0]);
        for col in state.iter() {
            meta.enable_equality(*col);
        }
        meta.enable_equality(partial_sbox);

        let poseidon_config = Pow5Chip::configure::<P128Pow5T3>(
            meta,
            state,
            partial_sbox,
            rc_a,
            rc_b,
        );

        meta.create_gate("coord bounding", |meta| {
            let s = meta.query_selector(s_coord);
            let p = meta.query_advice(advice_primes, Rotation::cur());
            let qx = meta.query_advice(advice_quotients_x, Rotation::cur());
            let cx = meta.query_advice(advice_coords_x, Rotation::cur());
            let qy = meta.query_advice(advice_quotients_y, Rotation::cur());
            let cy = meta.query_advice(advice_coords_y, Rotation::cur());
            
            let w = meta.query_fixed(fixed_width);
            let h = meta.query_fixed(fixed_height);

            vec![
                s.clone() * (p.clone() - (qx * w + cx)),
                s * (p - (qy * h + cy)),
            ]
        });

        meta.create_gate("bit extraction", |meta| {
            let s = meta.query_selector(s_extract);
            let pr = meta.query_advice(advice_pixel_r, Rotation::cur());
            let pg = meta.query_advice(advice_pixel_g, Rotation::cur());
            let pb = meta.query_advice(advice_pixel_b, Rotation::cur());
            
            let lr = meta.query_advice(advice_lsb_r, Rotation::cur());
            let lg = meta.query_advice(advice_lsb_g, Rotation::cur());
            let lb = meta.query_advice(advice_lsb_b, Rotation::cur());
            let expected = meta.query_advice(advice_recovered_bit, Rotation::cur());

            let two = Expression::Constant(Fp::from(2));
            let kappa = lr.clone() + lg.clone() - two.clone() * lr.clone() * lg.clone();
            let expected_calc = lb.clone() + kappa.clone() - two * lb.clone() * kappa;

            vec![
                s.clone() * (pr - lr.clone()), 
                s.clone() * (pg - lg.clone()),
                s.clone() * (pb - lb.clone()),
                s.clone() * (lr.clone() * (Expression::Constant(Fp::ONE) - lr)),
                s.clone() * (lg.clone() * (Expression::Constant(Fp::ONE) - lg)),
                s.clone() * (expected - expected_calc),
            ]
        });

        meta.create_gate("conditional swap", |meta| {
            let s = meta.query_selector(s_swap);
            let cur = meta.query_advice(advice_merkle_current, Rotation::cur());
            let sib = meta.query_advice(advice_merkle_sibling, Rotation::cur());
            let bit = meta.query_advice(advice_merkle_index_bit, Rotation::cur());
            let left = meta.query_advice(advice_swap_left, Rotation::cur());
            let right = meta.query_advice(advice_swap_right, Rotation::cur());

            let expected_left = cur.clone() + bit.clone() * (sib.clone() - cur.clone());
            let expected_right = cur.clone() + sib.clone() - expected_left.clone();

            vec![
                s.clone() * (bit.clone() * (Expression::Constant(Fp::ONE) - bit)),
                s.clone() * (left - expected_left),
                s * (right - expected_right),
            ]
        });

        ZkStegConfig {
            advice_primes,
            advice_quotients_x,
            advice_coords_x,
            advice_quotients_y,
            advice_coords_y,
            advice_pixel_r,
            advice_pixel_g,
            advice_pixel_b,
            advice_lsb_r,
            advice_lsb_g,
            advice_lsb_b,
            advice_recovered_bit,
            advice_merkle_current,
            advice_merkle_sibling,
            advice_merkle_index_bit,
            advice_swap_left,
            advice_swap_right,
            fixed_width,
            fixed_height,
            instance_expected_bits,
            instance_merkle_root,
            s_coord,
            s_extract,
            s_swap,
            poseidon_config,
        }
    }
}

fn assign_conditional_swap<'v>(
    region: &mut Region<'_, Fp>,
    config: &ZkStegConfig,
    row: usize,
    current: &AssignedCell<Fp, Fp>,
    sibling: &AssignedCell<Fp, Fp>,
    bit: &AssignedCell<Fp, Fp>,
) -> Result<(AssignedCell<Fp, Fp>, AssignedCell<Fp, Fp>), Error> {
    config.s_swap.enable(region, row)?;
    let current_here = current.copy_advice(|| "current", region, config.advice_merkle_current, row)?;
    let sibling_here = sibling.copy_advice(|| "sibling", region, config.advice_merkle_sibling, row)?;
    let bit_here = bit.copy_advice(|| "bit", region, config.advice_merkle_index_bit, row)?;

    let mut left_val = Value::unknown();
    let mut right_val = Value::unknown();

    current_here.value().zip(sibling_here.value()).zip(bit_here.value()).map(|((&c, &s), &b)| {
        if b == Fp::ZERO {
            left_val = Value::known(c);
            right_val = Value::known(s);
        } else {
            left_val = Value::known(s);
            right_val = Value::known(c);
        }
    });

    let left = region.assign_advice(|| "left", config.advice_swap_left, row, || left_val)?;
    let right = region.assign_advice(|| "right", config.advice_swap_right, row, || right_val)?;

    Ok((left, right))
}

#[derive(Clone, Default)]
pub struct ZkStegCircuit {
    pub image_width: u64,
    pub image_height: u64,
    pub samples: Vec<StegSampleWitness>,
}

impl Circuit<Fp> for ZkStegCircuit {
    type Config = ZkStegConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        ZkStegConfig::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<Fp>) -> Result<(), Error> {
        let mut recovered_cells = vec![];

        let mut padded_samples = self.samples.clone();
        if padded_samples.is_empty() {
            let dummy = StegSampleWitness {
                prime: 0, quotient_x: 0, coord_x: 0, quotient_y: 0, coord_y: 0,
                pixel_r: 0, pixel_g: 0, pixel_b: 0,
                expected_bit: Value::known(Fp::ZERO),
                merkle_path: [(Fp::ZERO, Fp::ZERO); MAX_TREE_DEPTH],
            };
            padded_samples.resize(MAX_SAMPLES, dummy);
        } else {
            let last = padded_samples.last().unwrap().clone();
            padded_samples.resize(MAX_SAMPLES, last);
        }

        layouter.assign_region(
            || "fixed dimensions",
            |mut region| {
                region.assign_fixed(|| "w", config.fixed_width, 0, || Value::known(Fp::from(self.image_width)))?;
                region.assign_fixed(|| "h", config.fixed_height, 0, || Value::known(Fp::from(self.image_height)))?;
                Ok(())
            },
        )?;

        for (i, sample) in padded_samples.iter().enumerate() {
            let (coord_x_cell, coord_y_cell, r_cell, g_cell, b_cell, assigned_recovered) = layouter.assign_region(
                || format!("sample {i}"),
                |mut region| {
                    config.s_coord.enable(&mut region, 0)?;
                    config.s_extract.enable(&mut region, 0)?;

                    region.assign_advice(|| "p", config.advice_primes, 0, || Value::known(Fp::from(sample.prime)))?;
                    region.assign_advice(|| "qx", config.advice_quotients_x, 0, || Value::known(Fp::from(sample.quotient_x)))?;
                    region.assign_advice(|| "qy", config.advice_quotients_y, 0, || Value::known(Fp::from(sample.quotient_y)))?;
                    
                    let cx = region.assign_advice(|| "cx", config.advice_coords_x, 0, || Value::known(Fp::from(sample.coord_x)))?;
                    let cy = region.assign_advice(|| "cy", config.advice_coords_y, 0, || Value::known(Fp::from(sample.coord_y)))?;

                    let pr = region.assign_advice(|| "pr", config.advice_pixel_r, 0, || Value::known(Fp::from(sample.pixel_r as u64)))?;
                    let pg = region.assign_advice(|| "pg", config.advice_pixel_g, 0, || Value::known(Fp::from(sample.pixel_g as u64)))?;
                    let pb = region.assign_advice(|| "pb", config.advice_pixel_b, 0, || Value::known(Fp::from(sample.pixel_b as u64)))?;

                    region.assign_advice(|| "lsb r", config.advice_lsb_r, 0, || Value::known(Fp::from((sample.pixel_r & 1) as u64)))?;
                    region.assign_advice(|| "lsb g", config.advice_lsb_g, 0, || Value::known(Fp::from((sample.pixel_g & 1) as u64)))?;
                    region.assign_advice(|| "lsb b", config.advice_lsb_b, 0, || Value::known(Fp::from((sample.pixel_b & 1) as u64)))?;

                    let rec = region.assign_advice(|| format!("recovered_bit_{}", i), config.advice_recovered_bit, 0, || sample.expected_bit)?;
                    
                    Ok((cx, cy, pr, pg, pb, rec))
                }
            )?;
            recovered_cells.push(assigned_recovered);

            let leaf_hasher = PoseidonHash::<Fp, Pow5Chip<Fp,3,2>, P128Pow5T3, ConstantLength<5>, 3, 2>::init(
                Pow5Chip::construct(config.poseidon_config.clone()),
                layouter.namespace(|| format!("leaf init {i}")),
            )?;
            let mut current = leaf_hasher.hash(
                layouter.namespace(|| format!("leaf hash {i}")),
                [coord_x_cell.clone(), coord_y_cell.clone(), r_cell.clone(), g_cell.clone(), b_cell.clone()],
            )?;

            for level in 0..MAX_TREE_DEPTH {
                let (sib_val, bit_val) = sample.merkle_path[level];
                let (sib_cell, bit_cell) = layouter.assign_region(
                    || format!("path witness {i} {level}"),
                    |mut region| {
                        let sib = region.assign_advice(|| "sib", config.advice_merkle_sibling, 0, || Value::known(sib_val))?;
                        let bit = region.assign_advice(|| "bit", config.advice_merkle_index_bit, 0, || Value::known(bit_val))?;
                        Ok((sib, bit))
                    },
                )?;

                let (left, right) = layouter.assign_region(
                    || format!("swap {i} {level}"),
                    |mut region| assign_conditional_swap(&mut region, &config, 0, &current, &sib_cell, &bit_cell),
                )?;

                let level_hasher = PoseidonHash::<Fp, Pow5Chip<Fp,3,2>, P128Pow5T3, ConstantLength<2>, 3, 2>::init(
                    Pow5Chip::construct(config.poseidon_config.clone()),
                    layouter.namespace(|| format!("level init {i} {level}")),
                )?;
                current = level_hasher.hash(layouter.namespace(|| format!("level exec {i} {level}")), [left, right])?;
            }

            layouter.constrain_instance(current.cell(), config.instance_merkle_root, 0)?;
        }

        for (i, cell) in recovered_cells.iter().enumerate().take(MAX_SAMPLES) {
            layouter.constrain_instance(cell.cell(), config.instance_expected_bits, i)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::dev::MockProver;
    use halo2_gadgets::poseidon::primitives::Hash as NativeHash;
    use pasta_curves::Fp;

    fn native_hash2(a: Fp, b: Fp) -> Fp {
        NativeHash::<Fp, P128Pow5T3, ConstantLength<2>, 3, 2>::init().hash([a, b])
    }
    
    fn native_leaf(x: Fp, y: Fp, r: Fp, g: Fp, b: Fp) -> Fp {
        NativeHash::<Fp, P128Pow5T3, ConstantLength<5>, 3, 2>::init().hash([x, y, r, g, b])
    }

    #[test]
    fn merkle_path_of_depth_4_verifies() {
        const DEPTH: usize = 4;
        let mut success_k = 0;
        
        let target_index_1 = 5usize;
        let target_index_2 = 10usize;
        let mut leaves = vec![Fp::from(0u64); 1 << DEPTH];
        
        // Leaf 1
        leaves[target_index_1] = native_leaf(Fp::from(3u64), Fp::from(2u64), Fp::from(1u64), Fp::from(0u64), Fp::from(1u64));
        
        // Leaf 2
        leaves[target_index_2] = native_leaf(Fp::from(5u64), Fp::from(1u64), Fp::from(0u64), Fp::from(1u64), Fp::from(0u64));

        let extract_path = |target_index| {
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
        let path2 = extract_path(target_index_2);

        // calculate root
        let mut cur_level = leaves.clone();
        for _ in 0..DEPTH {
            let mut next = vec![];
            for pair in cur_level.chunks(2) {
                next.push(native_hash2(pair[0], pair[1]));
            }
            cur_level = next;
        }
        let root = cur_level[0];

        let kappa1 = (1 & 1) ^ (0 & 1);
        let expected1 = (1 & 1) ^ kappa1;
        
        let kappa2 = (0 & 1) ^ (1 & 1);
        let expected2 = (0 & 1) ^ kappa2;

        let sample1 = StegSampleWitness {
            prime: 35, quotient_x: 2, coord_x: 3, quotient_y: 0, coord_y: 2, // 16x16
            pixel_r: 1, pixel_g: 0, pixel_b: 1,
            expected_bit: Value::known(Fp::from(expected1 as u64)),
            merkle_path: path1,
        };

        let sample2 = StegSampleWitness {
            prime: 21, quotient_x: 1, coord_x: 5, quotient_y: 0, coord_y: 1, // 16x16
            pixel_r: 0, pixel_g: 1, pixel_b: 0,
            expected_bit: Value::known(Fp::from(expected2 as u64)),
            merkle_path: path2,
        };

        let circuit = ZkStegCircuit {
            image_width: 16,
            image_height: 16,
            samples: vec![sample1.clone(), sample2],
        };

        let mut expected_bits_instance = vec![Fp::from(expected1 as u64), Fp::from(expected2 as u64)];
        expected_bits_instance.resize(MAX_SAMPLES, Fp::from(expected2 as u64));

        let public_instances = vec![expected_bits_instance.clone(), vec![root]];
        
        // Find correct k
        for test_k in 10..15 {
            match MockProver::run(test_k, &circuit, public_instances.clone()) {
                Ok(prover) => {
                    success_k = test_k;
                    prover.verify().unwrap();
                    break;
                },
                Err(e) => {
                    println!("Failed at k={}: {:?}", test_k, e);
                }
            }
        }
        assert!(success_k > 0, "Could not find a valid k");
        println!("Successfully verified with k={}", success_k);
        
        // Negative test 1: wrong public root
        let bad_root = vec![expected_bits_instance.clone(), vec![root + Fp::from(1u64)]];
        let bad_prover = MockProver::run(success_k, &circuit, bad_root).unwrap();
        assert!(bad_prover.verify().is_err(), "Must fail with wrong root");

        // Negative test 2: one sample has different root (fake path)
        let mut bad_path = path2;
        bad_path[0].0 = Fp::from(999);
        let sample2_bad = StegSampleWitness {
            prime: 21, quotient_x: 1, coord_x: 5, quotient_y: 0, coord_y: 1,
            pixel_r: 0, pixel_g: 1, pixel_b: 0,
            expected_bit: Value::known(Fp::from(expected2 as u64)),
            merkle_path: bad_path,
        };
        let bad_circuit = ZkStegCircuit {
            image_width: 16,
            image_height: 16,
            samples: vec![sample1, sample2_bad],
        };
        let bad_prover2 = MockProver::run(success_k, &bad_circuit, public_instances).unwrap();
        assert!(bad_prover2.verify().is_err(), "Must fail if paths resolve to different roots");
    }

    #[test]
    fn merkle_path_non_square_dimensions_verifies() {
        const DEPTH: usize = 4;
        let mut leaves = vec![Fp::from(0u64); 1 << DEPTH];
        let target_index = 5usize;
        leaves[target_index] = native_leaf(Fp::from(3u64), Fp::from(2u64), Fp::from(1u64), Fp::from(0u64), Fp::from(1u64));
        
        let extract_path = |target_index| {
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
        let path1 = extract_path(target_index);

        let mut cur_level = leaves.clone();
        for _ in 0..DEPTH {
            let mut next = vec![];
            for pair in cur_level.chunks(2) {
                next.push(native_hash2(pair[0], pair[1]));
            }
            cur_level = next;
        }
        let root = cur_level[0];
        let expected1 = (1 & 1) ^ ((1 & 1) ^ (0 & 1));

        let sample1 = StegSampleWitness {
            prime: 35, quotient_x: 2, coord_x: 3, quotient_y: 0, coord_y: 2,
            pixel_r: 1, pixel_g: 0, pixel_b: 1,
            expected_bit: Value::known(Fp::from(expected1 as u64)),
            merkle_path: path1,
        };

        // Try 512x256 dimensions
        let circuit = ZkStegCircuit {
            image_width: 512,
            image_height: 256,
            samples: vec![sample1.clone()],
        };

        let mut expected_bits_instance = vec![Fp::from(expected1 as u64)];
        expected_bits_instance.resize(MAX_SAMPLES, Fp::from(expected1 as u64));
        let public_instances = vec![expected_bits_instance, vec![root]];
        
        let prover = MockProver::run(14, &circuit, public_instances).unwrap();
        prover.verify().unwrap();
    }

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
        
        let extract_path = |target_index| {
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

        let kappa1 = (1 & 1) ^ (0 & 1);
        let expected1 = (1 & 1) ^ kappa1; // 1^1 = 0

        let sample1 = StegSampleWitness {
            prime: 35, quotient_x: 2, coord_x: 3, quotient_y: 0, coord_y: 2,
            pixel_r: 1, pixel_g: 0, pixel_b: 1,
            expected_bit: Value::known(Fp::from(expected1 as u64)),
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

        let mut expected_bits_instance = vec![Fp::from(expected1 as u64)];
        expected_bits_instance.resize(MAX_SAMPLES, Fp::from(expected1 as u64));
        
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

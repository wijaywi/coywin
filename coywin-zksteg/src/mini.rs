
use halo2_proofs::{
    circuit::{AssignedCell, Layouter, Region, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Expression, Selector, Instance},
    poly::Rotation,
};
use halo2_gadgets::poseidon::{
    Pow5Chip, Pow5Config, Hash as PoseidonHash,
    primitives::{P128Pow5T3, ConstantLength},
};
use ff::{Field, PrimeField};
use pasta_curves::Fp;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct MiniConfig {
    pub advice_merkle_current: Column<Advice>,
    pub advice_merkle_sibling: Column<Advice>,
    pub advice_merkle_index_bit: Column<Advice>,
    pub advice_swap_left: Column<Advice>,
    pub advice_swap_right: Column<Advice>,
    pub instance: Column<Instance>,
    pub poseidon_config: Pow5Config<Fp, 3, 2>,
    pub s_swap: Selector,
}

impl MiniConfig {
    pub fn configure(meta: &mut ConstraintSystem<Fp>) -> Self {
        let state: Vec<Column<Advice>> = (0..3).map(|_| meta.advice_column()).collect();
        let partial_sbox = meta.advice_column();
        let rc_a = (0..3).map(|_| meta.fixed_column()).collect::<Vec<_>>();
        let rc_b = (0..3).map(|_| meta.fixed_column()).collect::<Vec<_>>();
        meta.enable_constant(rc_b[0]);

        let poseidon_config = Pow5Chip::configure::<P128Pow5T3>(
            meta,
            state.try_into().unwrap(),
            partial_sbox,
            rc_a.try_into().unwrap(),
            rc_b.try_into().unwrap(),
        );

        let advice_merkle_current = meta.advice_column();
        let advice_merkle_sibling = meta.advice_column();
        let advice_merkle_index_bit = meta.advice_column();
        let advice_swap_left = meta.advice_column();
        let advice_swap_right = meta.advice_column();
        let instance = meta.instance_column();
        let s_swap = meta.selector();

        meta.enable_equality(advice_merkle_current);
        meta.enable_equality(advice_merkle_sibling);
        meta.enable_equality(advice_merkle_index_bit);
        meta.enable_equality(advice_swap_left);
        meta.enable_equality(advice_swap_right);
        meta.enable_equality(instance);

        meta.create_gate("conditional swap", |meta| {
            let s = meta.query_selector(s_swap);
            let cur = meta.query_advice(advice_merkle_current, Rotation::cur());
            let sib = meta.query_advice(advice_merkle_sibling, Rotation::cur());
            let bit = meta.query_advice(advice_merkle_index_bit, Rotation::cur());
            let left = meta.query_advice(advice_swap_left, Rotation::cur());
            let right = meta.query_advice(advice_swap_right, Rotation::cur());

            let bool_bit = bit.clone() * (Expression::Constant(Fp::ONE) - bit.clone());
            let expected_left = cur.clone() + bit.clone() * (sib.clone() - cur.clone());
            let expected_right = sib.clone() + bit.clone() * (cur - sib);

            vec![
                s.clone() * bool_bit,
                s.clone() * (left - expected_left),
                s * (right - expected_right),
            ]
        });

        MiniConfig {
            advice_merkle_current, advice_merkle_sibling, advice_merkle_index_bit,
            advice_swap_left, advice_swap_right, instance, poseidon_config, s_swap
        }
    }
}

pub fn assign_conditional_swap(
    region: &mut Region<Fp>,
    config: &MiniConfig,
    row: usize,
    current: &AssignedCell<Fp, Fp>,
    sibling: &AssignedCell<Fp, Fp>,
    bit: &AssignedCell<Fp, Fp>,
) -> Result<(AssignedCell<Fp, Fp>, AssignedCell<Fp, Fp>), Error> {
    let current_here = current.copy_advice(|| "current", region, config.advice_merkle_current, row)?;
    let bit_here = bit.copy_advice(|| "bit", region, config.advice_merkle_index_bit, row)?;

    config.s_swap.enable(region, row)?;

    let left_val = current_here.value().zip(sibling.value()).zip(bit_here.value())
        .map(|((&c, &s), &b)| c + b * (s - c));
    let right_val = current_here.value().zip(sibling.value()).zip(bit_here.value())
        .map(|((&c, &s), &b)| s + b * (c - s));

    let left = region.assign_advice(|| "swap_left", config.advice_swap_left, row, || left_val)?;
    let right = region.assign_advice(|| "swap_right", config.advice_swap_right, row, || right_val)?;

    Ok((left, right))
}

#[derive(Default)]
pub struct MiniCircuit {
    pub a: Value<Fp>,
    pub b: Value<Fp>,
    pub bit0: Value<Fp>,
    pub bit1: Value<Fp>,
}

impl Circuit<Fp> for MiniCircuit {
    type Config = MiniConfig;
    type FloorPlanner = SimpleFloorPlanner;
    
    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        MiniConfig::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<Fp>) -> Result<(), Error> {
        let chip = Pow5Chip::construct(config.poseidon_config.clone());
        
        let (a_cell, b_cell, bit0_cell, bit1_cell) = layouter.assign_region(
            || "init",
            |mut region| {
                let a = region.assign_advice(|| "a", config.advice_merkle_current, 0, || self.a)?;
                let b = region.assign_advice(|| "b", config.advice_merkle_sibling, 0, || self.b)?;
                let bit0 = region.assign_advice(|| "bit0", config.advice_merkle_index_bit, 0, || self.bit0)?;
                let bit1 = region.assign_advice(|| "bit1", config.advice_merkle_index_bit, 1, || self.bit1)?;
                Ok((a, b, bit0, bit1))
            }
        )?;

        let (left0, right0) = layouter.assign_region(
            || "swap0",
            |mut region| {
                let b_sib = b_cell.copy_advice(|| "b_sib", &mut region, config.advice_merkle_sibling, 0)?;
                assign_conditional_swap(&mut region, &config, 0, &a_cell, &b_sib, &bit0_cell)
            }
        )?;

        let hasher0 = PoseidonHash::<Fp, Pow5Chip<Fp, 3, 2>, P128Pow5T3, ConstantLength<2>, 3, 2>::init(
            Pow5Chip::construct(config.poseidon_config.clone()),
            layouter.namespace(|| "hash0"),
        )?;
        let hash0_out = hasher0.hash(layouter.namespace(|| "hash0 exec"), [left0, right0])?;

        layouter.constrain_instance(hash0_out.cell(), config.instance, 0)?;

        let (left1, right1) = layouter.assign_region(
            || "swap1",
            |mut region| {
                let b_sib = b_cell.copy_advice(|| "b_sib", &mut region, config.advice_merkle_sibling, 0)?;
                assign_conditional_swap(&mut region, &config, 0, &hash0_out, &b_sib, &bit1_cell)
            }
        )?;

        let hasher1 = PoseidonHash::<Fp, Pow5Chip<Fp, 3, 2>, P128Pow5T3, ConstantLength<2>, 3, 2>::init(
            Pow5Chip::construct(config.poseidon_config.clone()),
            layouter.namespace(|| "hash1"),
        )?;
        let hash1_out = hasher1.hash(layouter.namespace(|| "hash1 exec"), [left1, right1])?;

        layouter.constrain_instance(hash1_out.cell(), config.instance, 1)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::dev::MockProver;
    use halo2_gadgets::poseidon::primitives::Hash as NativeHash;

    #[test]
    fn test_mini_swap_bit1_matches_native_reversed_order() {
        let a = Fp::from(10);
        let b = Fp::from(20);
        
        let expected_hash0 = NativeHash::<Fp, P128Pow5T3, ConstantLength<2>, 3, 2>::init().hash([a, b]);
        let expected_hash1 = NativeHash::<Fp, P128Pow5T3, ConstantLength<2>, 3, 2>::init().hash([b, expected_hash0]);

        let circuit = MiniCircuit {
            a: Value::known(a),
            b: Value::known(b),
            bit0: Value::known(Fp::from(0)),
            bit1: Value::known(Fp::from(1)),
        };
        let k = 8;
        
        let prover = MockProver::run(k, &circuit, vec![vec![expected_hash0, expected_hash1]]).unwrap();
        prover.verify().unwrap(); 

        let wrong_order = NativeHash::<Fp, P128Pow5T3, ConstantLength<2>, 3, 2>::init().hash([expected_hash0, b]);
        let bad = MockProver::run(k, &circuit, vec![vec![expected_hash0, wrong_order]]).unwrap();
        assert!(bad.verify().is_err(), "test ini HARUS gagal kalau urutan swap terbalik");
    }
}



use halo2_proofs::{
    circuit::{AssignedCell, Layouter, Region, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Expression, Selector},
    poly::Rotation,
};
use halo2_gadgets::poseidon::{
    Pow5Chip, Pow5Config, Hash as PoseidonHash,
    primitives::{P128Pow5T3, ConstantLength},
};
use pasta_curves::Fp;
use ff::PrimeField;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct MiniConfig {
    pub advice_merkle_current: Column<Advice>,
    pub advice_merkle_sibling: Column<Advice>,
    pub advice_merkle_index_bit: Column<Advice>,
    pub advice_swap_left: Column<Advice>,
    pub advice_swap_right: Column<Advice>,
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
        let s_swap = meta.selector();

        meta.enable_equality(advice_merkle_current);
        meta.enable_equality(advice_merkle_sibling);
        meta.enable_equality(advice_merkle_index_bit);
        meta.enable_equality(advice_swap_left);
        meta.enable_equality(advice_swap_right);

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
            advice_swap_left, advice_swap_right, poseidon_config, s_swap
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


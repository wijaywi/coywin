use std::marker::PhantomData;
use ff::PrimeField;
use halo2_proofs::{
    circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value},
    plonk::{
        Advice, Circuit, Column, ConstraintSystem, Error, Expression, Fixed, Instance, Selector,
    },
    poly::Rotation,
};

/// Configuration containing advice columns, fixed tables, and selectors for zk-Steg.
#[derive(Clone, Debug)]
pub struct ZkStegConfig {
    pub advice_primes: Column<Advice>,
    pub advice_coords_x: Column<Advice>,
    pub advice_coords_y: Column<Advice>,
    pub advice_quotients_x: Column<Advice>,
    pub advice_quotients_y: Column<Advice>,

    pub advice_pixel_r: Column<Advice>,
    pub advice_pixel_g: Column<Advice>,
    pub advice_pixel_b: Column<Advice>,
    pub advice_lsb_r: Column<Advice>,
    pub advice_lsb_g: Column<Advice>,
    pub advice_lsb_b: Column<Advice>,
    pub advice_recovered_bit: Column<Advice>,

    pub fixed_width: Column<Fixed>,
    pub fixed_height: Column<Fixed>,

    pub instance: Column<Instance>,

    pub s_coord: Selector,
    pub s_extract: Selector,
}

impl ZkStegConfig {
    pub fn configure<F: PrimeField>(meta: &mut ConstraintSystem<F>) -> Self {
        let advice_primes = meta.advice_column();
        let advice_coords_x = meta.advice_column();
        let advice_coords_y = meta.advice_column();
        let advice_quotients_x = meta.advice_column();
        let advice_quotients_y = meta.advice_column();

        let advice_pixel_r = meta.advice_column();
        let advice_pixel_g = meta.advice_column();
        let advice_pixel_b = meta.advice_column();
        let advice_lsb_r = meta.advice_column();
        let advice_lsb_g = meta.advice_column();
        let advice_lsb_b = meta.advice_column();
        let advice_recovered_bit = meta.advice_column();

        let fixed_width = meta.fixed_column();
        let fixed_height = meta.fixed_column();
        let instance = meta.instance_column();

        let s_coord = meta.selector();
        let s_extract = meta.selector();

        meta.enable_equality(instance);
        meta.enable_equality(advice_recovered_bit);
        meta.enable_equality(advice_primes);

        // GATE 1: Prime Modulo Coordinate Derivation Gate (C_Prime)
        meta.create_gate("prime_coordinate_modulo_gate", |meta| {
            let s = meta.query_selector(s_coord);
            let prime = meta.query_advice(advice_primes, Rotation::cur());
            let q_x = meta.query_advice(advice_quotients_x, Rotation::cur());
            let x_coord = meta.query_advice(advice_coords_x, Rotation::cur());
            let width = meta.query_fixed(fixed_width);

            let q_y = meta.query_advice(advice_quotients_y, Rotation::cur());
            let y_coord = meta.query_advice(advice_coords_y, Rotation::cur());
            let height = meta.query_fixed(fixed_height);

            let constraint_x = prime.clone() - (q_x.clone() * width + x_coord);
            let constraint_y = q_x - (q_y * height + y_coord);

            vec![
                s.clone() * constraint_x,
                s * constraint_y,
            ]
        });

        // GATE 2: LSB Extraction & Dynamic XOR Decryption Gate (C_Extract)
        meta.create_gate("lsb_dynamic_xor_extraction_gate", |meta| {
            let s = meta.query_selector(s_extract);

            let lsb_r = meta.query_advice(advice_lsb_r, Rotation::cur());
            let lsb_g = meta.query_advice(advice_lsb_g, Rotation::cur());
            let lsb_b = meta.query_advice(advice_lsb_b, Rotation::cur());
            let recovered_bit = meta.query_advice(advice_recovered_bit, Rotation::cur());

            let bool_r = lsb_r.clone() * (Expression::Constant(F::ONE) - lsb_r.clone());
            let bool_g = lsb_g.clone() * (Expression::Constant(F::ONE) - lsb_g.clone());
            let bool_b = lsb_b.clone() * (Expression::Constant(F::ONE) - lsb_b.clone());
            let bool_m = recovered_bit.clone() * (Expression::Constant(F::ONE) - recovered_bit.clone());

            let two = Expression::Constant(F::from(2));
            let kappa = lsb_r.clone() + lsb_g.clone() - two.clone() * lsb_r * lsb_g;
            let expected_recovered_bit = lsb_b.clone() + kappa.clone() - two * lsb_b * kappa;
            let xor_constraint = recovered_bit - expected_recovered_bit;

            vec![
                s.clone() * bool_r,
                s.clone() * bool_g,
                s.clone() * bool_b,
                s.clone() * bool_m,
                s * xor_constraint,
            ]
        });

        ZkStegConfig {
            advice_primes, advice_coords_x, advice_coords_y, advice_quotients_x, advice_quotients_y,
            advice_pixel_r, advice_pixel_g, advice_pixel_b, advice_lsb_r, advice_lsb_g, advice_lsb_b,
            advice_recovered_bit, fixed_width, fixed_height, instance, s_coord, s_extract,
        }
    }
}

#[derive(Clone)]
pub struct StegSampleWitness<F: PrimeField> {
    pub prime: u64,
    pub quotient_x: u64,
    pub coord_x: u64,
    pub quotient_y: u64,
    pub coord_y: u64,
    pub pixel_r: u8,
    pub pixel_g: u8,
    pub pixel_b: u8,
    pub expected_bit: Value<F>,
}

#[derive(Clone, Default)]
pub struct ZkStegCircuit<F: PrimeField> {
    pub image_width: u64,
    pub image_height: u64,
    pub samples: Vec<StegSampleWitness<F>>,
    pub _marker: PhantomData<F>,
}

impl<F: PrimeField> Circuit<F> for ZkStegCircuit<F> {
    type Config = ZkStegConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self {
            image_width: self.image_width,
            image_height: self.image_height,
            samples: vec![],
            _marker: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        ZkStegConfig::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let mut recovered_cells: Vec<AssignedCell<F, F>> = Vec::with_capacity(self.samples.len());

        layouter.assign_region(
            || "zk-Steg Verification Matrix",
            |mut region| {
                for (i, sample) in self.samples.iter().enumerate() {
                    config.s_coord.enable(&mut region, i)?;
                    config.s_extract.enable(&mut region, i)?;

                    region.assign_fixed(|| format!("width_{}", i), config.fixed_width, i, || Value::known(F::from(self.image_width)))?;
                    region.assign_fixed(|| format!("height_{}", i), config.fixed_height, i, || Value::known(F::from(self.image_height)))?;

                    region.assign_advice(|| format!("prime_{}", i), config.advice_primes, i, || Value::known(F::from(sample.prime)))?;
                    region.assign_advice(|| format!("quotient_x_{}", i), config.advice_quotients_x, i, || Value::known(F::from(sample.quotient_x)))?;
                    region.assign_advice(|| format!("coord_x_{}", i), config.advice_coords_x, i, || Value::known(F::from(sample.coord_x)))?;
                    region.assign_advice(|| format!("quotient_y_{}", i), config.advice_quotients_y, i, || Value::known(F::from(sample.quotient_y)))?;
                    region.assign_advice(|| format!("coord_y_{}", i), config.advice_coords_y, i, || Value::known(F::from(sample.coord_y)))?;

                    let lsb_r = sample.pixel_r & 1;
                    let lsb_g = sample.pixel_g & 1;
                    let lsb_b = sample.pixel_b & 1;

                    region.assign_advice(|| format!("pixel_r_{}", i), config.advice_pixel_r, i, || Value::known(F::from(sample.pixel_r as u64)))?;
                    region.assign_advice(|| format!("pixel_g_{}", i), config.advice_pixel_g, i, || Value::known(F::from(sample.pixel_g as u64)))?;
                    region.assign_advice(|| format!("pixel_b_{}", i), config.advice_pixel_b, i, || Value::known(F::from(sample.pixel_b as u64)))?;

                    region.assign_advice(|| format!("lsb_r_{}", i), config.advice_lsb_r, i, || Value::known(F::from(lsb_r as u64)))?;
                    region.assign_advice(|| format!("lsb_g_{}", i), config.advice_lsb_g, i, || Value::known(F::from(lsb_g as u64)))?;
                    region.assign_advice(|| format!("lsb_b_{}", i), config.advice_lsb_b, i, || Value::known(F::from(lsb_b as u64)))?;

                    let assigned_recovered = region.assign_advice(|| format!("recovered_bit_{}", i), config.advice_recovered_bit, i, || sample.expected_bit)?;
                    recovered_cells.push(assigned_recovered);
                }
                Ok(())
            },
        )?;

        for (i, cell) in recovered_cells.iter().enumerate().take(32) {
            layouter.constrain_instance(cell.cell(), config.instance, i)?;
        }

        Ok(())
    }
}

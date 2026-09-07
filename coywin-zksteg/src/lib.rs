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

pub const MAX_SAMPLES: usize = 32;

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
        let dummy_samples = vec![
            StegSampleWitness {
                prime: 0,
                quotient_x: 0,
                coord_x: 0,
                quotient_y: 0,
                coord_y: 0,
                pixel_r: 0,
                pixel_g: 0,
                pixel_b: 0,
                expected_bit: Value::known(F::ZERO),
            }; MAX_SAMPLES
        ];

        Self {
            image_width: self.image_width,
            image_height: self.image_height,
            samples: dummy_samples,
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
        let mut recovered_cells: Vec<AssignedCell<F, F>> = Vec::with_capacity(MAX_SAMPLES);

        // Pad samples up to MAX_SAMPLES
        let mut padded_samples = self.samples.clone();
        padded_samples.resize(
            MAX_SAMPLES,
            StegSampleWitness {
                prime: 0,
                quotient_x: 0,
                coord_x: 0,
                quotient_y: 0,
                coord_y: 0,
                pixel_r: 0,
                pixel_g: 0,
                pixel_b: 0,
                expected_bit: Value::known(F::ZERO),
            }
        );

        layouter.assign_region(
            || "zk-Steg Verification Matrix",
            |mut region| {
                for (i, sample) in padded_samples.iter().enumerate() {
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

        for (i, cell) in recovered_cells.iter().enumerate().take(MAX_SAMPLES) {
            layouter.constrain_instance(cell.cell(), config.instance, i)?;
        }

        Ok(())
    }
}

use halo2_proofs::plonk::{create_proof, keygen_pk, keygen_vk, verify_proof, ProvingKey, VerifyingKey, VerificationStrategy, SingleVerifier};
use halo2_proofs::poly::commitment::Params;
use halo2_proofs::transcript::{Blake2bRead, Blake2bWrite, TranscriptRead, TranscriptWrite, Challenge255};
use pasta_curves::{pallas, vesta, EqAffine, Fp};
use rand::rngs::OsRng;

pub fn generate_params(k: u32) -> Params<vesta::Affine> {
    Params::<vesta::Affine>::new(k)
}

pub fn generate_keys(params: &Params<vesta::Affine>, circuit: &ZkStegCircuit<Fp>) -> Result<(ProvingKey<vesta::Affine>, VerifyingKey<vesta::Affine>), halo2_proofs::plonk::Error> {
    let vk = keygen_vk(params, &circuit.without_witnesses())?;
    let pk = keygen_pk(params, vk.clone(), circuit)?;
    Ok((pk, vk))
}

pub fn create_steg_proof(
    params: &Params<vesta::Affine>,
    pk: &ProvingKey<vesta::Affine>,
    circuit: ZkStegCircuit<Fp>,
    public_instances: &[&[Fp]],
) -> Result<Vec<u8>, halo2_proofs::plonk::Error> {
    let mut transcript = Blake2bWrite::<_, vesta::Affine, Challenge255<_>>::init(vec![]);
    create_proof(
        params,
        pk,
        &[circuit],
        &[public_instances],
        OsRng,
        &mut transcript,
    )?;
    Ok(transcript.finalize())
}

pub fn verify_steg_proof(
    params: &Params<vesta::Affine>,
    vk: &VerifyingKey<vesta::Affine>,
    proof: &[u8],
    public_instances: &[&[Fp]],
) -> bool {
    let strategy = SingleVerifier::new(params);
    let mut transcript = Blake2bRead::<_, _, Challenge255<_>>::init(proof);
    verify_proof(
        params,
        vk,
        strategy,
        &[public_instances],
        &mut transcript,
    ).is_ok()
}


pub fn reconstruct_vk(
    params: &Params<vesta::Affine>,
    image_width: u64,
    image_height: u64,
) -> Result<VerifyingKey<vesta::Affine>, halo2_proofs::plonk::Error> {
    let circuit = ZkStegCircuit::<Fp> {
        image_width,
        image_height,
        samples: vec![],
        _marker: std::marker::PhantomData,
    }.without_witnesses(); 

    keygen_vk(params, &circuit)
}


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



pub mod mini;

pub mod full_merkle;

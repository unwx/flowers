use crate::art::dynamic::DynNoise;
use crate::noise::theory::{random_noise, NoiseOptions};
use crate::Random;
use anyhow::{bail, Result};
use rand::{Rng, SeedableRng};
use std::ops::RangeInclusive;

pub trait NoiseFactory {
    // TODO Docs
    //  (N, f32):
    //  - N: noise
    //  - f32: noise_scale
    fn noise(
        &self,
        noise_output_seed: u32,
        noise_structure_seed: u64,
    ) -> Result<(DynNoise<f64, 2>, f32)>;
}


#[derive(Debug, Clone)]
pub struct RandomNoiseFactory {
    basic_noises: RangeInclusive<u16>,
    fractal_noises: RangeInclusive<u16>,
    octaves_per_fractal: RangeInclusive<u16>,
    decorate_probability: RangeInclusive<f32>,
    noise_scale: RangeInclusive<f32>,
}

impl RandomNoiseFactory {
    pub fn try_new(
        basic_noises: RangeInclusive<u16>,
        fractal_noises: RangeInclusive<u16>,
        octaves_per_fractal: RangeInclusive<u16>,
        decorate_probability: RangeInclusive<f32>,
        noise_scale: RangeInclusive<f32>,
    ) -> Result<Self> {
        if basic_noises.is_empty() {
            bail!("'basic_noises' cannot be empty");
        }
        if fractal_noises.is_empty() {
            bail!("'fractal_noises' cannot be empty");
        }
        if octaves_per_fractal.is_empty() {
            bail!("'octaves_per_fractal' cannot be empty");
        }
        if decorate_probability.is_empty() {
            bail!("'decorate_probability' cannot be empty");
        }
        if noise_scale.is_empty() {
            bail!("'noise_scale' cannot be empty");
        }

        if *noise_scale.start() <= 0.0 {
            bail!("'noise_scale' must be > 0.0");
        }

        Ok(Self {
            basic_noises,
            fractal_noises,
            octaves_per_fractal,
            decorate_probability,
            noise_scale,
        })
    }

    pub fn basic_noises(&self) -> RangeInclusive<u16> {
        self.basic_noises.clone()
    }

    pub fn fractal_noises(&self) -> RangeInclusive<u16> {
        self.fractal_noises.clone()
    }

    pub fn octaves_per_fractal(&self) -> RangeInclusive<u16> {
        self.octaves_per_fractal.clone()
    }

    pub fn decorate_probability(&self) -> RangeInclusive<f32> {
        self.decorate_probability.clone()
    }

    pub fn noise_scale(&self) -> RangeInclusive<f32> {
        self.noise_scale.clone()
    }
}

impl NoiseFactory for RandomNoiseFactory {
    fn noise(
        &self,
        noise_output_seed: u32,
        noise_structure_seed: u64,
    ) -> Result<(DynNoise<f64, 2>, f32)> {
        let mut random = Random::seed_from_u64(noise_structure_seed);

        let noise_scale = random.gen_range(self.noise_scale());
        let noise = random_noise(
            NoiseOptions::default()
                .with_basic(random.gen_range(self.basic_noises()))
                .with_fractals(
                    random.gen_range(self.fractal_noises()),
                    random.gen_range(self.octaves_per_fractal()),
                )
                .with_decorate_probability(random.gen_range(self.decorate_probability())),
            noise_output_seed,
            &mut random,
        );

        Ok((DynNoise::new(noise), noise_scale))
    }
}

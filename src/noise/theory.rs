use crate::noise::dynamic::DynNoise;
use noise::core::worley::{distance_functions, ReturnType};
use noise::{
    Abs, BasicMulti, Billow, Fbm, HybridMulti, Max, Min, MultiFractal, Negate, Perlin,
    PerlinSurflet, RidgedMulti, Simplex, SuperSimplex, Turbulence, Worley,
};
use rand::Rng;

#[derive(Debug, Copy, Clone)]
pub struct NoiseOptions {
    basic: u16,
    fractals: u16,
    octaves_per_fractal: u16,
    decorate_probability: f32,
}

impl NoiseOptions {
    pub fn with_basic(self, basic_noises_count: u16) -> Self {
        Self {
            basic: basic_noises_count.max(1),
            ..self
        }
    }

    pub fn with_fractals(self, fractal_noises_count: u16, octaves_per_fractal_count: u16) -> Self {
        Self {
            fractals: fractal_noises_count,
            octaves_per_fractal: if fractal_noises_count != 0 {
                octaves_per_fractal_count.clamp(2, 16)
            } else {
                0
            },
            ..self
        }
    }

    pub fn with_decorate_probability(self, decorate_probability: f32) -> Self {
        Self {
            decorate_probability: decorate_probability.clamp(0.0, 1.0),
            ..self
        }
    }
}

impl Default for NoiseOptions {
    fn default() -> Self {
        Self {
            basic: 1,
            fractals: 0,
            octaves_per_fractal: 0,
            decorate_probability: 0.0,
        }
    }
}


pub fn random_noise<R: Rng>(options: NoiseOptions, seed: u32, random: &mut R) -> DynNoise {
    fn decorate_noise<R: Rng>(noise: DynNoise, random: &mut R) -> DynNoise {
        let noise = Box::new(noise);
        match random.gen_range(0..=2) {
            0 => DynNoise::from(Abs::new(noise)),
            1 => DynNoise::from(Negate::new(noise)),
            2 => DynNoise::from(
                Turbulence::new(noise)
                    .set_frequency(random.gen_range(1.0..=1.7))
                    .set_power(random.gen_range(1.0..=1.7))
                    .set_roughness(random.gen_range(2..=4)),
            ),
            _ => unreachable!(),
        }
    }

    fn maybe_decorate_noise<R: Rng>(noise: DynNoise, probability: f32, random: &mut R) -> DynNoise {
        if probability > random.gen_range(0.0..1.0) {
            decorate_noise(noise, random)
        } else {
            noise
        }
    }


    fn random_basic_noise<R: Rng>(
        seed: u32,
        decorate_probability: f32,
        random: &mut R,
    ) -> DynNoise {
        let noise = match random.gen_range(0..=4) {
            0 => DynNoise::from(Perlin::new(seed)),
            1 => DynNoise::from(PerlinSurflet::new(seed)),
            2 => DynNoise::from(Simplex::new(seed)),
            3 => DynNoise::from(SuperSimplex::new(seed)),
            4 => DynNoise::from(
                Worley::new(seed)
                    .set_frequency(random.gen_range(1.0..=2.5))
                    .set_return_type(if random.gen_bool(0.5) {
                        ReturnType::Value
                    } else {
                        ReturnType::Distance
                    })
                    .set_distance_function(match random.gen_range(0..=3) {
                        0 => distance_functions::euclidean,
                        1 => distance_functions::euclidean_squared,
                        2 => distance_functions::manhattan,
                        3 => distance_functions::chebyshev,
                        _ => unreachable!(),
                    }),
            ),
            _ => unreachable!(),
        };

        maybe_decorate_noise(noise, decorate_probability, random)
    }

    fn random_basic_noises<R: Rng>(
        seed: u32,
        decorate_probability: f32,
        count: u16,
        random: &mut R,
    ) -> Vec<DynNoise> {
        (0..count)
            .map(|_| random_basic_noise(seed, decorate_probability, random))
            .collect()
    }


    fn random_fractal_noise<R: Rng>(
        seed: u32,
        octaves: Vec<DynNoise>,
        decorate_probability: f32,
        random: &mut R,
    ) -> DynNoise {
        let frequency = random.gen_range(0.8..=2.0);
        let lacunarity = random.gen_range(1.0..=1.8);
        let persistence = random.gen_range(0.15..=0.4);
        let octaves: Vec<Box<DynNoise>> = octaves.into_iter().map(Box::new).collect();

        let noise = match random.gen_range(0..=4) {
            0 => DynNoise::from(
                Fbm::new(seed)
                    .set_octaves(octaves.len())
                    .set_frequency(frequency)
                    .set_lacunarity(lacunarity)
                    .set_persistence(persistence)
                    .set_sources(octaves),
            ),
            1 => DynNoise::from(
                Billow::new(seed)
                    .set_octaves(octaves.len())
                    .set_frequency(frequency)
                    .set_lacunarity(lacunarity)
                    .set_persistence(persistence)
                    .set_sources(octaves),
            ),
            2 => DynNoise::from(
                BasicMulti::new(seed)
                    .set_octaves(octaves.len())
                    .set_frequency(frequency)
                    .set_lacunarity(lacunarity)
                    .set_persistence(persistence)
                    .set_sources(octaves),
            ),
            3 => DynNoise::from(
                HybridMulti::new(seed)
                    .set_octaves(octaves.len())
                    .set_frequency(frequency)
                    .set_lacunarity(lacunarity)
                    .set_persistence(persistence)
                    .set_sources(octaves),
            ),
            4 => DynNoise::from(
                RidgedMulti::new(seed)
                    .set_octaves(octaves.len())
                    .set_frequency(frequency)
                    .set_lacunarity(lacunarity)
                    .set_persistence(persistence)
                    .set_attenuation(random.gen_range(1.8..=3.0))
                    .set_sources(octaves),
            ),
            _ => unreachable!(),
        };

        maybe_decorate_noise(noise, decorate_probability, random)
    }


    let mut noises = Vec::with_capacity((options.basic + options.fractals) as usize);
    noises.extend(random_basic_noises(
        seed,
        options.decorate_probability,
        options.basic,
        random,
    ));
    noises.extend((0..options.fractals).map(|_| {
        let octaves = random_basic_noises(
            seed,
            options.decorate_probability,
            options.octaves_per_fractal,
            random,
        );
        random_fractal_noise(seed, octaves, options.decorate_probability, random)
    }));


    fn merge_noises<R: Rng>(mut noises: Vec<DynNoise>, random: &mut R) -> DynNoise {
        assert!(
            !noises.is_empty(),
            "cannot merge Vec<DynNoise> into DynNoise: Vec is empty"
        );

        while noises.len() > 1 {
            let first = Box::new(noises.pop().expect("noises must have at least 2 elements"));
            let second = Box::new(noises.pop().expect("noises must have at least 1 element"));

            let merged = match random.gen_range(0..=1) {
                0 => DynNoise::from(Min::new(first, second)),
                1 => DynNoise::from(Max::new(first, second)),
                _ => unreachable!(),
            };

            noises.push(merged);
        }

        noises
            .pop()
            .expect("noises cannot be empty, and must contain merged DynNoise")
    }

    merge_noises(noises, random)
}

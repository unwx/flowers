use crate::common::MyRc;
use crate::graphics::noise::DynNoise;
use crate::math::real::debug_eval_finite;
use noise::core::worley::{distance_functions, ReturnType};
use noise::{
    Abs, Add, BasicMulti, Billow, Curve, Fbm, HybridMulti, Max, Min, MultiFractal, Multiply,
    Negate, Perlin, PerlinSurflet, RidgedMulti, RotatePoint, Simplex, SuperSimplex, Terrace,
    Turbulence, Worley,
};
use rand::distributions::uniform::SampleRange;
use rand::Rng;

#[must_use]
pub fn random_noise<R: Rng>(internal_seed: u32, random: &mut R) -> DynNoise {
    fn rc<T>(value: T) -> MyRc<T> {
        MyRc::new(value)
    }

    #[rustfmt::skip]
    fn decorate_noise<R: Rng>(
        noise: DynNoise,
        decoration_chance: f32,
        random: &mut R,
    ) -> DynNoise {
        if random.gen_range(0.0..=1.0) < decoration_chance {
            return noise;
        }

        match random.gen_range(0..6) {
            0 => DynNoise::from(Abs::new(rc(noise))),
            1 => DynNoise::from(Negate::new(rc(noise))),
            2 => DynNoise::from(RotatePoint::new(rc(noise)).set_angles(
                random.gen_range(-180.0..=180.0),
                random.gen_range(-180.0..=180.0),
                random.gen_range(-180.0..=180.0),
                random.gen_range(-180.0..=180.0),
            )),
            3 => DynNoise::from(
                Turbulence::new(rc(noise))
                    .set_frequency(random.gen_range(0.01..=7.5))
                    .set_power(random.gen_range(0.1..=5.0))
                    .set_roughness(random.gen_range(2..6)),
            ),
            4 => {
                let mut curve = Curve::new(rc(noise));
                let range = -3.0..=3.0;

                let points = rand_unique_f64_values(
                    random.gen_range(4..=32),
                    random,
                    || range.clone()
                );

                for point in points {
                    curve = curve.add_control_point(point, random.gen_range(range.clone()));
                }

                DynNoise::from(curve)
            }
            5 => {
                let mut terrace = Terrace::new(rc(noise)).invert_terraces(random.gen_bool(0.5));
                let points = rand_unique_f64_values(
                    random.gen_range(2..=32),
                    random,
                    || -3.0..=3.0
                );

                for point in points {
                    terrace = terrace.add_control_point(point);
                }

                DynNoise::from(terrace)
            }
            _ => unreachable!(),
        }
    }

    fn gen_noise<R: Rng>(
        seed: u32,
        fractal_chance: &mut f32,
        fractal_chance_reduction: f32,
        decoration_chance: f32,
        random: &mut R,
    ) -> DynNoise {
        fn gen_fractal_sources<R: Rng>(
            size: usize,
            seed: u32,
            fractal_chance: &mut f32,
            fractal_chance_reduction: f32,
            decoration_chance: f32,
            random: &mut R,
        ) -> Vec<MyRc<DynNoise>> {
            let mut sources = Vec::with_capacity(size);
            for _ in 0..size {
                *fractal_chance = (*fractal_chance - fractal_chance_reduction).clamp(0.0, 1.0);
                let noise = gen_noise(
                    seed,
                    fractal_chance,
                    fractal_chance_reduction,
                    decoration_chance,
                    random,
                );

                sources.push(rc(noise));
            }

            sources
        }

        let noise = {
            if random.gen_range(0.0..=1.0) < *fractal_chance {
                let octaves = random.gen_range(2..=16);
                let sources = gen_fractal_sources(
                    octaves,
                    seed,
                    fractal_chance,
                    fractal_chance_reduction,
                    decoration_chance,
                    random,
                );

                match random.gen_range(0..5) {
                    0 => DynNoise::from(
                        Fbm::new(seed)
                            .set_octaves(octaves)
                            .set_frequency(random.gen_range(0.01..=7.5))
                            .set_lacunarity(random.gen_range(1.0..=3.0))
                            .set_persistence(random.gen_range(0.2..=0.8))
                            .set_sources(sources),
                    ),
                    1 => DynNoise::from(
                        Billow::new(seed)
                            .set_octaves(octaves)
                            .set_frequency(random.gen_range(0.01..=7.5))
                            .set_lacunarity(random.gen_range(1.0..=3.0))
                            .set_persistence(random.gen_range(0.2..=0.8))
                            .set_sources(sources),
                    ),
                    2 => DynNoise::from(
                        BasicMulti::new(seed)
                            .set_octaves(octaves)
                            .set_frequency(random.gen_range(0.01..=7.5))
                            .set_lacunarity(random.gen_range(1.0..=3.0))
                            .set_persistence(random.gen_range(0.2..=0.8))
                            .set_sources(sources),
                    ),
                    3 => DynNoise::from(
                        HybridMulti::new(seed)
                            .set_octaves(octaves)
                            .set_frequency(random.gen_range(0.01..=7.5))
                            .set_lacunarity(random.gen_range(1.0..=3.0))
                            .set_persistence(random.gen_range(0.2..=0.8))
                            .set_sources(sources),
                    ),
                    4 => DynNoise::from(
                        RidgedMulti::new(seed)
                            .set_octaves(octaves)
                            .set_frequency(random.gen_range(0.01..=7.5))
                            .set_lacunarity(random.gen_range(1.0..=3.0))
                            .set_persistence(random.gen_range(0.2..=0.8))
                            .set_attenuation(random.gen_range(0.1..=3.0))
                            .set_sources(sources),
                    ),
                    _ => unreachable!(),
                }
            } else {
                match random.gen_range(0..5) {
                    0 => DynNoise::from(Perlin::new(seed)),
                    1 => DynNoise::from(PerlinSurflet::new(seed)),
                    2 => DynNoise::from(Simplex::new(seed)),
                    3 => DynNoise::from(SuperSimplex::new(seed)),
                    4 => DynNoise::from(
                        Worley::new(seed)
                            .set_frequency(random.gen_range(0.01..7.5))
                            .set_return_type(if random.gen_bool(0.5) {
                                ReturnType::Value
                            } else {
                                ReturnType::Distance
                            })
                            .set_distance_function(match random.gen_range(0..4) {
                                0 => distance_functions::euclidean,
                                1 => distance_functions::euclidean_squared,
                                2 => distance_functions::manhattan,
                                3 => distance_functions::chebyshev,
                                _ => unreachable!(),
                            }),
                    ),
                    _ => unreachable!(),
                }
            }
        };

        decorate_noise(noise, decoration_chance, random)
    }

    fn merge_noises<R: Rng>(mut noises: Vec<DynNoise>, random: &mut R) -> DynNoise {
        if noises.is_empty() {
            return DynNoise::default();
        }

        while noises.len() > 1 {
            let len = noises.len();
            let first = rc(std::mem::take(&mut noises[len - 1]));
            let second = rc(std::mem::take(&mut noises[len - 2]));

            let merged = match random.gen_range(0..4) {
                0 => DynNoise::from(Add::new(first, second)),
                1 => DynNoise::from(Multiply::new(first, second)),
                2 => DynNoise::from(Min::new(first, second)),
                3 => DynNoise::from(Max::new(first, second)),
                _ => unreachable!(),
            };

            noises.remove(len - 1);
            noises[len - 2] = merged;
        }

        noises.into_iter().next().unwrap()
    }

    let decoration_chance = random.gen_range(0.0..=1.0);
    let mut fractal_chance = random.gen_range(0.0..=1.0);
    let fractal_chance_reduction = random.gen_range(0.1..=1.0);
    let fractal_chance_ref = &mut fractal_chance;

    let mut noises = Vec::with_capacity(random.gen_range(1..16));
    for _ in 0..noises.capacity() {
        noises.push(gen_noise(
            internal_seed,
            fractal_chance_ref,
            fractal_chance_reduction,
            decoration_chance,
            random,
        ));
    }

    merge_noises(noises, random)
}

#[must_use]
fn rand_unique_f64_values<Rand, Range, RangeFn>(
    size: usize,
    random: &mut Rand,
    range: RangeFn,
) -> Vec<f64>
where
    Rand: Rng,
    Range: SampleRange<f64>,
    RangeFn: Fn() -> Range,
{
    let mut values: Vec<f64> = Vec::with_capacity(size);
    let mut failures = 0;

    while values.len() < size {
        let value = debug_eval_finite!(random.gen_range(range()));
        if values.iter().any(|x| (x - value).abs() < f64::EPSILON) {
            failures += 1;
            debug_assert!(failures < size * 10);
            assert!(
                failures < size * 100,
                "bug: infinite loop detected in rand_unique_f64_values() method"
            )
        } else {
            values.push(value);
        }
    }

    values
}

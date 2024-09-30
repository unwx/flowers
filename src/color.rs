use crate::math::normalize_f64;
use colorgrad::{
    BasisGradient, BlendMode, CatmullRomGradient, Color, Gradient, GradientBuilder, LinearGradient,
    SharpGradient,
};
use dyn_clone::{clone_trait_object, DynClone};
use glam::I16Vec2;
use noise::core::worley::{distance_functions, ReturnType};
use noise::utils::{NoiseMapBuilder, PlaneMapBuilder};
use noise::{
    BasicMulti, Billow, Curve, Fbm, HybridMulti, MultiFractal, NoiseFn, Perlin, PerlinSurflet,
    RidgedMulti, RotatePoint, Seedable, Simplex, SuperSimplex, Terrace, Turbulence, Worley,
};
use palette::color_theory::{Analogous, Complementary, SplitComplementary, Tetradic, Triadic};
use palette::convert::FromColorUnclamped;
use palette::{Hsl, Srgb};
use rand::distributions::uniform::SampleRange;
use rand::Rng;
use std::cmp::Ordering;

/*
 * Gradient
 */

pub struct DynGradient {
    gradient: Box<dyn Gradient>,
}

impl DynGradient {
    pub fn new<G: Gradient + 'static>(gradient: G) -> Self {
        Self {
            gradient: Box::new(gradient),
        }
    }
}

impl Gradient for DynGradient {
    fn at(&self, t: f32) -> Color {
        self.gradient.at(t)
    }

    fn repeat_at(&self, t: f32) -> Color {
        self.gradient.repeat_at(t)
    }

    fn reflect_at(&self, t: f32) -> Color {
        self.gradient.reflect_at(t)
    }

    fn domain(&self) -> (f32, f32) {
        self.gradient.domain()
    }

    fn colors(&self, n: usize) -> Vec<Color> {
        self.gradient.colors(n)
    }

    fn sharp(&self, segment: u16, smoothness: f32) -> SharpGradient {
        self.gradient.sharp(segment, smoothness)
    }
}

/*
 * Noise
 */

pub const MAX_OCTAVES: usize = 16;

pub trait CloneableNoise<T, const DIM: usize>: NoiseFn<T, DIM> + DynClone {}
clone_trait_object!(CloneableNoise<f64, 3>);

impl CloneableNoise<f64, 3> for DynNoise<f64, 3> {}
impl CloneableNoise<f64, 3> for Perlin {}
impl CloneableNoise<f64, 3> for PerlinSurflet {}
impl CloneableNoise<f64, 3> for Simplex {}
impl CloneableNoise<f64, 3> for SuperSimplex {}
impl CloneableNoise<f64, 3> for Worley {}

impl CloneableNoise<f64, 3> for Fbm<DynNoise<f64, 3>> {}
impl CloneableNoise<f64, 3> for Billow<DynNoise<f64, 3>> {}
impl CloneableNoise<f64, 3> for BasicMulti<DynNoise<f64, 3>> {}
impl CloneableNoise<f64, 3> for HybridMulti<DynNoise<f64, 3>> {}
impl CloneableNoise<f64, 3> for RidgedMulti<DynNoise<f64, 3>> {}

impl CloneableNoise<f64, 3> for noise::Abs<f64, DynNoise<f64, 3>, 3> {}
impl CloneableNoise<f64, 3> for noise::Negate<f64, DynNoise<f64, 3>, 3> {}
impl CloneableNoise<f64, 3> for RotatePoint<DynNoise<f64, 3>> {}
impl CloneableNoise<f64, 3> for Turbulence<DynNoise<f64, 3>, DynNoise<f64, 3>> {}
impl CloneableNoise<f64, 3> for Curve<f64, DynNoise<f64, 3>, 3> {}
impl CloneableNoise<f64, 3> for Terrace<f64, DynNoise<f64, 3>, 3> {}

impl CloneableNoise<f64, 3> for noise::Add<f64, DynNoise<f64, 3>, DynNoise<f64, 3>, 3> {}
impl CloneableNoise<f64, 3> for noise::Multiply<f64, DynNoise<f64, 3>, DynNoise<f64, 3>, 3> {}
impl CloneableNoise<f64, 3> for noise::Power<f64, DynNoise<f64, 3>, DynNoise<f64, 3>, 3> {}
impl CloneableNoise<f64, 3> for noise::Min<f64, DynNoise<f64, 3>, DynNoise<f64, 3>, 3> {}
impl CloneableNoise<f64, 3> for noise::Max<f64, DynNoise<f64, 3>, DynNoise<f64, 3>, 3> {}

pub struct DynNoise<T, const DIM: usize> {
    noise: Box<dyn CloneableNoise<T, DIM>>,
}

impl<T, const DIM: usize> DynNoise<T, DIM> {
    pub fn new<N: CloneableNoise<T, DIM> + 'static>(noise: N) -> Self {
        Self {
            noise: Box::new(noise),
        }
    }
}

impl<const DIM: usize> NoiseFn<f64, DIM> for DynNoise<f64, DIM> {
    fn get(&self, point: [f64; DIM]) -> f64 {
        // Prevent noise::math::vectors::Vector3<T>::floor_to_isize -> unwrap() panic
        let mut safe_point: [f64; DIM] = [0.0; DIM];
        fn clamp(value: f64) -> f64 {
            value.clamp(isize::MIN as f64, isize::MAX as f64)
        }

        for i in 0..DIM {
            safe_point[i] = clamp(point[i]);
        }

        clamp(self.noise.get(safe_point))
    }
}

impl Default for DynNoise<f64, 3> {
    fn default() -> Self {
        DynNoise::new(Perlin::default())
    }
}

impl Clone for DynNoise<f64, 3> {
    fn clone(&self) -> Self {
        Self {
            noise: self.noise.clone(),
        }
    }
}

/// [Seedable] is not object-safe, but is also required for fractal noise.
/// This is a small workaround.
impl<T, const DIM: usize> Seedable for DynNoise<T, DIM> {
    fn set_seed(self, _: u32) -> Self {
        self
    }

    fn seed(&self) -> u32 {
        0
    }
}

/*
 * Methods
 */

pub fn colorize<G, N>(
    area: &[(I16Vec2, I16Vec2)],
    gradient: &G,
    noise: &N,
    noise_scale: f32,
) -> Option<(Vec<(I16Vec2, Color)>, Color)>
where
    G: Gradient,
    N: NoiseFn<f64, 3>,
{
    if area.is_empty() {
        return None;
    }

    let (min_y, max_y, min_x, max_x, total_elements) = {
        let min_y = area.first()?.0.y;
        let max_y = area.last()?.1.y;
        debug_assert!(min_y <= max_y);

        let mut min_x = i16::MAX;
        let mut max_x = i16::MIN;
        let mut total_elements = 0;

        for (from, to) in area {
            debug_assert_eq!(from.y, to.y);
            debug_assert!(from.x <= to.x);

            min_x = min_x.min(from.x);
            max_x = max_x.max(to.x);
            total_elements += (to.x - from.x) as usize + 1;
        }

        (min_y, max_y, min_x, max_x, total_elements)
    };
    if min_x == i16::MAX || max_x == i16::MIN {
        return None;
    }

    let mut pixels = Vec::with_capacity(total_elements);
    let noise_map = PlaneMapBuilder::new(&noise)
        .set_size((max_x - min_x) as usize, (max_y - min_y) as usize)
        .set_x_bounds(f64::from(-noise_scale.abs()), f64::from(noise_scale.abs()))
        .set_y_bounds(f64::from(-noise_scale.abs()), f64::from(noise_scale.abs()))
        .build();

    let min_noise = *noise_map
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))?;
    let max_noise = *noise_map
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))?;
    let mut average_noise = 0.0_f64;

    for (from, to) in area {
        let y = from.y;

        for x in from.x..=to.x {
            let noise_value = normalize_f64(
                noise_map.get_value((x - min_x) as usize, (y - min_y) as usize),
                min_noise,
                max_noise,
                f64::from(gradient.domain().0),
                f64::from(gradient.domain().1),
            ) as f32;

            let gradient_value = gradient.at(noise_value);
            average_noise += noise_value as f64;

            pixels.push((I16Vec2::new(x, y), gradient_value));
        }
    }

    average_noise /= total_elements as f64;
    pixels.shrink_to_fit();
    Some((pixels, gradient.at(average_noise as f32)))
}

pub fn random_gradient<R: Rng>(colors: &[Color], random: &mut R) -> DynGradient {
    assert!(
        colors.len() > 1,
        "To build a gradient, you must provide at least two colors"
    );
    let blending_mode = match random.gen_range(0..3) {
        0 => BlendMode::Rgb,
        1 => BlendMode::LinearRgb,
        2 => BlendMode::Oklab,
        _ => unreachable!(),
    };

    let mut builder = GradientBuilder::new();
    let stages = builder.mode(blending_mode).colors(colors);

    match random.gen_range(0..4) {
        0 => stages.build::<LinearGradient>().map(DynGradient::new),
        1 => stages.build::<BasisGradient>().map(DynGradient::new),
        2 => stages.build::<CatmullRomGradient>().map(DynGradient::new),
        3 => stages
            .build::<LinearGradient>()
            .map(|g| g.sharp(random.gen_range(2..=32), random.gen_range(0.0..=1.0)))
            .map(DynGradient::new),
        _ => unreachable!(),
    }
    .unwrap()
}

pub fn random_noise<R: Rng>(internal_seed: u32, random: &mut R) -> DynNoise<f64, 3> {
    #[rustfmt::skip]
    fn decorate_noise<R: Rng>(
        noise: DynNoise<f64, 3>,
        decoration_chance: f32,
        random: &mut R,
    ) -> DynNoise<f64, 3> {
        if random.gen_range(0.0..=1.0) < decoration_chance {
            return noise;
        }

        match random.gen_range(0..6) {
            0 => DynNoise::new(noise::Abs::new(noise)),
            1 => DynNoise::new(noise::Negate::new(noise)),
            2 => DynNoise::new(RotatePoint::new(noise).set_angles(
                random.gen_range(-180.0..=180.0),
                random.gen_range(-180.0..=180.0),
                random.gen_range(-180.0..=180.0),
                random.gen_range(-180.0..=180.0),
            )),
            3 => DynNoise::new(
                Turbulence::new(noise)
                    .set_frequency(random.gen_range(0.01..=7.5))
                    .set_power(random.gen_range(0.1..=5.0))
                    .set_roughness(random.gen_range(2..6)),
            ),
            4 => {
                let mut curve = Curve::new(noise);
                let bound = 3.0;

                let points = rand_unique_f64_values(
                    random.gen_range(4..=32),
                    random,
                    || -bound..=bound
                );

                for point in points {
                    curve = curve.add_control_point(point, random.gen_range(-bound..=bound));
                }

                DynNoise::new(curve)
            }
            5 => {
                let mut terrace = Terrace::new(noise).invert_terraces(random.gen_bool(0.5));
                let points = rand_unique_f64_values(
                    random.gen_range(2..=32),
                    random,
                    || -3.0..=3.0
                );

                for point in points {
                    terrace = terrace.add_control_point(point);
                }

                DynNoise::new(terrace)
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
    ) -> DynNoise<f64, 3> {
        fn gen_noises<R: Rng>(
            size: usize,
            seed: u32,
            fractal_chance: &mut f32,
            fractal_chance_reduction: f32,
            decoration_chance: f32,
            random: &mut R,
        ) -> Vec<DynNoise<f64, 3>> {
            let mut noises = Vec::with_capacity(size);
            for _ in 0..size {
                *fractal_chance = (*fractal_chance - fractal_chance_reduction).clamp(0.0, 1.0);
                noises.push(gen_noise(
                    seed,
                    fractal_chance,
                    fractal_chance_reduction,
                    decoration_chance,
                    random,
                ));
            }

            noises
        }

        let noise = {
            if random.gen_range(0.0..=1.0) < *fractal_chance {
                let octaves = random.gen_range(2..=MAX_OCTAVES);
                let sources = gen_noises(
                    octaves,
                    seed,
                    fractal_chance,
                    fractal_chance_reduction,
                    decoration_chance,
                    random,
                );

                match random.gen_range(0..5) {
                    0 => DynNoise::new(
                        Fbm::new(seed)
                            .set_octaves(octaves)
                            .set_frequency(random.gen_range(0.01..=7.5))
                            .set_lacunarity(random.gen_range(1.0..=3.0))
                            .set_persistence(random.gen_range(0.2..=0.8))
                            .set_sources(sources),
                    ),
                    1 => DynNoise::new(
                        Billow::new(seed)
                            .set_octaves(octaves)
                            .set_frequency(random.gen_range(0.01..=7.5))
                            .set_lacunarity(random.gen_range(1.0..=3.0))
                            .set_persistence(random.gen_range(0.2..=0.8))
                            .set_sources(sources),
                    ),
                    2 => DynNoise::new(
                        BasicMulti::new(seed)
                            .set_octaves(octaves)
                            .set_frequency(random.gen_range(0.01..=7.5))
                            .set_lacunarity(random.gen_range(1.0..=3.0))
                            .set_persistence(random.gen_range(0.2..=0.8))
                            .set_sources(sources),
                    ),
                    3 => DynNoise::new(
                        HybridMulti::new(seed)
                            .set_octaves(octaves)
                            .set_frequency(random.gen_range(0.01..=7.5))
                            .set_lacunarity(random.gen_range(1.0..=3.0))
                            .set_persistence(random.gen_range(0.2..=0.8))
                            .set_sources(sources),
                    ),
                    4 => DynNoise::new(
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
                    0 => DynNoise::new(Perlin::new(seed)),
                    1 => DynNoise::new(PerlinSurflet::new(seed)),
                    2 => DynNoise::new(Simplex::new(seed)),
                    3 => DynNoise::new(SuperSimplex::new(seed)),
                    4 => DynNoise::new(
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

    fn merge_noises<R: Rng>(mut noises: Vec<DynNoise<f64, 3>>, random: &mut R) -> DynNoise<f64, 3> {
        if noises.is_empty() {
            return DynNoise::default();
        }
        if noises.len() == 1 {
            return noises[0].clone();
        }

        while noises.len() > 1 {
            let len = noises.len();
            let first = noises[len - 1].clone();
            let second = noises[len - 2].clone();

            let merged = match random.gen_range(0..4) {
                0 => DynNoise::new(noise::Add::new(first, second)),
                1 => DynNoise::new(noise::Multiply::new(first, second)),
                2 => DynNoise::new(noise::Min::new(first, second)),
                3 => DynNoise::new(noise::Max::new(first, second)),
                _ => unreachable!(),
            };

            noises.remove(len - 1);
            noises[len - 2] = merged;
        }

        noises[0].clone()
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

/*
 * Color
 */

pub const MIN_COLOR_LIGHT: f32 = 0.1;
pub const MAX_COLOR_LIGHT: f32 = 0.9;

pub fn random_color<R: Rng>(random: &mut R) -> Color {
    hsl_to_color(Hsl::new_srgb(
        random.gen_range(0.0..=1.0),
        random.gen_range(0.0..=1.0),
        random.gen_range(MIN_COLOR_LIGHT..=MAX_COLOR_LIGHT),
    ))
}

pub fn random_background_color<R: Rng>(primary_color: Color, random: &mut R) -> Color {
    let primary_color = color_to_hsl(primary_color);

    let hsl = if primary_color.lightness > 0.15 {
        Hsl::new_srgb(
            primary_color.hue,
            primary_color.saturation,
            random.gen_range(0.95..=1.0),
        )
    } else {
        Hsl::new_srgb(
            primary_color.hue,
            primary_color.saturation,
            random.gen_range(0.0..0.05),
        )
    };

    hsl_to_color(hsl)
}

pub fn random_palette<R: Rng>(size: usize, primary_color: Color, random: &mut R) -> Vec<Color> {
    assert!(
        size <= 12,
        "palette can only contain up to 12 colors, size: {size}"
    );

    if size <= 1 {
        return vec![primary_color];
    }

    let mut palette = Vec::with_capacity(size + 3);
    palette.push(color_to_hsl(primary_color));

    while palette.len() < size {
        let base_color = palette[random.gen_range(0..palette.len())];
        let split = match random.gen_range(0..5) {
            0 => vec![base_color.complementary()],
            1 => {
                let c = base_color.split_complementary();
                vec![c.0, c.1]
            }
            2 => {
                let c = base_color.analogous();
                vec![c.0, c.1]
            }
            3 => {
                let c = base_color.triadic();
                vec![c.0, c.1]
            }
            4 => {
                let c = base_color.tetradic();
                vec![c.0, c.1, c.2]
            }
            _ => unreachable!(),
        };

        for color in split.iter().take(split.len().min(size - palette.len())) {
            /*
             * I assume that the `size` parameter of the palette will not be large enough, so Vec O(n) will suffice.
             * Otherwise we will probably have to implement a custom HslHash structure with Hash impl.
             */
            if !palette.contains(color) {
                palette.push(color.clone());
            }
        }
    }

    palette.sort_unstable_by(|hsl1, hsl2| {
        f32::from(hsl1.hue)
            .partial_cmp(&f32::from(hsl2.hue))
            .unwrap_or(Ordering::Equal)
    });

    palette.iter().map(|hsl: &Hsl| hsl_to_color(*hsl)).collect()
}

pub fn hsl_to_color(hsl: Hsl) -> Color {
    let srgb = Srgb::from_color_unclamped(hsl);
    rgb_to_color(srgb)
}

pub fn rgb_to_color(srgb: Srgb) -> Color {
    Color::new(srgb.red, srgb.green, srgb.blue, 1.0)
}

pub fn color_to_rgb(color: Color) -> Srgb {
    Srgb::new(color.r, color.g, color.b)
}

pub fn color_to_hsl(color: Color) -> Hsl {
    Hsl::from_color_unclamped(color_to_rgb(color))
}

pub fn color_to_image_rgb(color: Color) -> image::Rgb<u8> {
    let srgb = color_to_rgb(color).into_format::<u8>();
    image::Rgb::from([srgb.red, srgb.green, srgb.blue])
}

pub fn image_rgb_to_color(rgb: image::Rgb<u8>) -> Color {
    let srgb = Srgb::new(rgb.0[0], rgb.0[1], rgb.0[2]).into_format::<f32>();
    rgb_to_color(srgb)
}


/// O(n) warning, small `size` expected.
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
        let value: f64 = random.gen_range(range());
        if values.iter().any(|x| (x - value).abs() < f64::EPSILON) {
            failures += 1;
            assert!(
                failures < size * 100,
                "infinite loop detected in rand_unique_f64_values() method"
            )
        } else {
            values.push(value);
        }
    }

    values
}

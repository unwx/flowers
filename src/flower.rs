use crate::graphics::color::convert::{hsl_to_rgb, rgb_to_hsl};
use crate::graphics::color::gradient::{random_gradient, DynGradient};
use crate::graphics::color::theory::{
    random_color, random_palette, MAX_COLOR_LIGHT, MIN_COLOR_LIGHT,
};
use crate::graphics::image::Raster;
use crate::graphics::noise::theory::random_noise;
use crate::graphics::noise::DynNoise;
use crate::graphics::render::color_area;
use crate::math::area::{cull, find_inner_area, Area};
use crate::math::curve::{eval_polar_sin, merge, scale, MergeMode};
use crate::math::real::{debug_assert_finite, debug_eval_finite};
use crate::math::{interpolate, normalize};
use crate::mosaic;
use crate::mosaic::{random_mosaic, Mosaic};
use crate::rand::RestorableRng;
use anyhow::{bail, Context, Result};
use glam::{I16Vec2, Mat2, Vec2};
use palette::rgb::Rgb;
use rand::prelude::SliceRandom;
use rand::{Rng, RngCore};
use std::f32::consts::PI;
use std::ops::RangeInclusive;
use std::rc::Rc;

#[derive(Clone)]
pub struct Flower {
    mosaic: Mosaic,
    petals: Vec<Raster>,
    radius: u16,
}

impl Flower {
    #[must_use]
    pub fn new(mosaic: Mosaic, petals: Vec<Raster>, radius: u16) -> Self {
        Self {
            mosaic,
            petals,
            radius,
        }
    }

    #[must_use]
    pub fn mosaic(&self) -> &Mosaic {
        &self.mosaic
    }

    #[must_use]
    pub fn petals(&self) -> &Vec<Raster> {
        &self.petals
    }

    #[must_use]
    pub fn radius(&self) -> u16 {
        self.radius
    }
}

/*
 * Internal structures
 */

#[derive(Debug, Copy, Clone)]
enum PetalFunction {
    Sin,
    Tan,
}

#[derive(Debug, Copy, Clone)]
enum PetalArrangement {
    Valvate {
        max_interpetal_angle_delta: f32,
    },
    RadiallySymmetrical {
        max_interpetal_angle_delta: f32,
        petals_count: u16,
    },
}

#[derive(Debug, Copy, Clone)]
struct RandomValue {
    value: f32,
    max_delta: f32,
}

impl RandomValue {
    #[must_use]
    fn from_range<R: Rng>(range: RangeInclusive<f32>, random: &mut R) -> Self {
        Self::from_min_max(*range.start(), *range.end(), random)
    }

    #[must_use]
    fn from_min_max<R: Rng>(min_value: f32, max_value: f32, random: &mut R) -> Self {
        debug_assert_finite!(min_value, max_value);
        assert!(min_value <= max_value, "min_value must be <= max_value");

        let value = random.gen_range(min_value..=max_value);
        let max_delta = random.gen_range(0.0..=(value - min_value).min(max_value - value));
        debug_assert_finite!(value, max_delta);

        Self { value, max_delta }
    }

    #[must_use]
    fn get<R: Rng>(self, random: &mut R) -> f32 {
        debug_eval_finite!(self.value + random.gen_range(-self.max_delta..=self.max_delta))
    }
}

#[derive(Clone)]
struct RandomGradient {
    palette: Vec<Rgb>,
    gradient: Option<Rc<DynGradient>>,
}

impl RandomGradient {
    #[must_use]
    fn get<R: Rng>(&self, random: &mut R) -> Rc<DynGradient> {
        self.gradient.clone().unwrap_or_else(|| {
            Rc::new(random_gradient(
                self.palette.as_slice(),
                &mut RestorableRng::new(random.next_u64()),
            ))
        })
    }
}

#[derive(Clone)]
struct RandomNoise {
    noise_seed: Option<u64>,
    noise: Option<Rc<DynNoise>>,
}

impl RandomNoise {
    #[must_use]
    fn get<R: Rng>(&self, random: &mut R) -> Rc<DynNoise> {
        self.noise.clone().unwrap_or_else(|| {
            Rc::new(random_noise(
                random.next_u32(),
                &mut RestorableRng::new(self.noise_seed.unwrap_or_else(|| random.next_u64())),
            ))
        })
    }
}

#[derive(Debug, Clone)]
struct LayerGenOptions {
    flip: bool,
    mirror: bool,
    mirror_direction: RandomValue,
    petal_function: PetalFunction,
    petal_arrangement: PetalArrangement,
    petal_distance_from_origin: u16,
    k: RandomValue,
    size: RandomValue,
}

#[derive(Debug, Copy, Clone)]
struct PetalOptions {
    k: f32,
    flip: bool,
    mirror: bool,
    function: PetalFunction,
}

struct Petal {
    skeleton: Vec<I16Vec2>,
    area: Area,
}

impl Petal {
    pub fn new(skeleton: Vec<I16Vec2>, area: Area) -> Self {
        Self { skeleton, area }
    }
}

/*
 * Methods
 */

pub const MIN_RADIUS: u16 = 100;
pub const MAX_RADIUS: u16 = (i16::MAX as u16 / 2) - 1;
const MAX_LAYERS: usize = 12;

pub fn random_flower(radius: u16, random: &mut RestorableRng) -> Result<Flower> {
    {
        assert!(
            (MIN_RADIUS..=MAX_RADIUS).contains(&radius),
            "invalid radius ({radius}), allowed: [{MIN_RADIUS} <= radius <= {MAX_RADIUS}]",
        );
    }

    let mosaic_radius_range = {
        let from = (radius as f32 * 0.03).round() as u16;
        let to = (radius as f32 * 0.40).round() as u16;
        from..=to
    };
    let mosaic_radius = random
        .gen_range(mosaic_radius_range.clone())
        .clamp(mosaic::MIN_RADIUS, mosaic::MAX_RADIUS);
    let mosaic = random_mosaic(mosaic_radius, &mut random.restore())
        .context("failed to generate the mosaic for the center of the flower")?;

    let layers_count = random.gen_range(1..=MAX_LAYERS);
    let mut layers = Vec::with_capacity(layers_count);

    let gradient = {
        let palette = if random.gen_bool(0.5) {
            mosaic.palette().clone()
        } else {
            random_palette(
                random.gen_range(2..=6),
                mosaic
                    .palette()
                    .first()
                    .copied()
                    .unwrap_or_else(|| random_color(random)),
                random,
            )
        };

        let gradient = random.gen_option(|r| Rc::new(random_gradient(&palette, r)));
        RandomGradient { palette, gradient }
    };
    let noise = RandomNoise {
        noise_seed: random.gen_option(|r| r.next_u64()),
        noise: random.gen_option(|r| Rc::new(random_noise(r.next_u32(), r))),
    };

    let flip = random.gen_bool(0.5);
    let mirror = random.gen_bool(0.5);
    let mirror_direction = RandomValue {
        value: random.gen_range(-0.1..=0.1),
        max_delta: if random.gen_bool(0.5) { 1.0 } else { 0.0 },
    };
    let petal_function = if random.gen_bool(0.5) {
        PetalFunction::Sin
    } else {
        PetalFunction::Tan
    };

    let k_range = 1.1..=6.0;
    let k = RandomValue::from_range(k_range.clone(), random);
    let noise_scale = random.gen_range(0.1..=7.5);

    let petal_distance_from_origin_range = 0..=((mosaic_radius as f32 * 0.8) as u16);
    let petal_distance_from_origin = random.gen_range(petal_distance_from_origin_range.clone());
    let min_petal_length = {
        let max_initial_length = radius as f32 * 0.8;
        let min_length = (mosaic_radius as f32 * 1.1)
            .max((radius as f32) * 0.03)
            .max(10.0)
            .min(max_initial_length);

        debug_assert!(min_length <= max_initial_length);
        random.gen_range(min_length..=max_initial_length) as u16
    };

    for i in 0..layers_count {
        let gen_options = LayerGenOptions {
            flip,
            mirror,
            mirror_direction,
            petal_function,
            petal_arrangement: {
                let max_interpetal_angle_delta = random.gen_range(0.0..=(PI / 6.0));
                match random.gen_range(0..2) {
                    0 => PetalArrangement::Valvate {
                        max_interpetal_angle_delta,
                    },
                    1 => PetalArrangement::RadiallySymmetrical {
                        max_interpetal_angle_delta,
                        petals_count: {
                            let normalized_count =
                                normalize(k.value, *k_range.start(), *k_range.end(), 10.0, 40.0);
                            let factor = normalize(
                                petal_distance_from_origin as f32,
                                *petal_distance_from_origin_range.start() as f32,
                                *mosaic_radius_range.end() as f32,
                                1.0,
                                3.0,
                            );

                            let max_petals_count = debug_eval_finite!(normalized_count * factor);
                            random.gen_range((max_petals_count / 3.0)..=max_petals_count) as u16
                        },
                    },
                    _ => unreachable!(),
                }
            },
            petal_distance_from_origin,
            k,
            size: {
                let value = {
                    if layers_count == 1 || min_petal_length == radius {
                        radius as f32
                    } else {
                        normalize(
                            i as f32,
                            0.0,
                            (layers_count - 1) as f32,
                            min_petal_length as f32,
                            radius as f32,
                        )
                    }
                };

                let space_available = radius as f32 - min_petal_length as f32;
                debug_assert!(space_available > 0.0);

                let max_delta = {
                    let interpetal_delta = (space_available / layers_count as f32) / 2.0;
                    debug_assert_finite!(interpetal_delta);

                    interpetal_delta
                        .min(value - min_petal_length as f32)
                        .min(radius as f32 - value)
                };

                let max_delta = random.gen_range(0.0..=max_delta);
                RandomValue { value, max_delta }
            },
        };

        let layer = random_layer(&gen_options, random).context(format!(
            "failed to generate a flower petal layer with the following parameters: {:?}",
            &gen_options
        ))?;
        layers.push(layer);
    }

    shuffle_petals(&mut layers, random);
    optimize_layers(&mut layers);
    assert!(
        !layers.is_empty(),
        "bug: each layer was removed during optimization"
    );

    let mut rasters = draw_layers(layers, &gradient, &noise, noise_scale, random);
    rasters.reverse();

    Ok(Flower::new(mosaic, rasters, radius))
}

#[must_use]
fn draw_layers<R: Rng>(
    layers: Vec<Vec<Petal>>,
    gradient: &RandomGradient,
    noise: &RandomNoise,
    noise_scale: f32,
    random: &mut R,
) -> Vec<Raster> {
    if layers.is_empty() {
        return vec![];
    }

    debug_assert_finite!(noise_scale);
    let light_reduction = {
        let min = 0.015;
        let max = normalize(layers.len() as f32, 0.0, MAX_LAYERS as f32, 0.03, min);
        random.gen_range(min..=max)
    };

    let layers_count = layers.len();
    let mut rasters = Vec::new();

    for (layer_index, layer) in layers.into_iter().enumerate() {
        for petal in layer {
            let mut pixels = color_area(
                &petal.area,
                gradient.get(random).as_ref(),
                noise.get(random).as_ref(),
                noise_scale,
            );

            for (_, rgb) in &mut pixels {
                let mut hsl = rgb_to_hsl(*rgb);
                let current_light_reduction = light_reduction * layer_index as f32;

                hsl.lightness -= current_light_reduction;
                hsl.lightness = hsl.lightness.clamp(
                    {
                        let max_reduction = light_reduction * ((layers_count - 1) as f32);
                        MIN_COLOR_LIGHT + max_reduction - current_light_reduction
                    },
                    MAX_COLOR_LIGHT,
                );
                *rgb = hsl_to_rgb(hsl)
            }

            rasters.push(Raster::new(petal.skeleton, pixels));
        }
    }

    rasters
}

fn optimize_layers(layers: &mut Vec<Vec<Petal>>) {
    enum CullingResult {
        NoAreaVisible,
        WholeAreaVisible,
        Culled(Area),
    }

    fn cull_back_area(back_area: &Area, front_areas: &[&Area]) -> CullingResult {
        let mut result = CullingResult::WholeAreaVisible;
        for front_area in front_areas {
            if !back_area.intersects(front_area) {
                continue;
            }

            let visible_area = {
                match result {
                    CullingResult::NoAreaVisible => {
                        return CullingResult::NoAreaVisible;
                    }
                    CullingResult::WholeAreaVisible => cull(back_area, front_area),
                    CullingResult::Culled(area) => cull(&area, front_area),
                }
            };
            if let Some(area) = visible_area {
                result = CullingResult::Culled(area);
            } else {
                return CullingResult::NoAreaVisible;
            }
        }

        result
    }

    for i in 0..layers.len() {
        for y in (0..=i).rev() {
            let mut petal_index = layers[i].len();
            while let Some(next) = petal_index.checked_sub(1) {
                petal_index = next;

                let mut lower_or_same_layer: Vec<&Petal> = layers[y].iter().collect();
                if i == y {
                    lower_or_same_layer = lower_or_same_layer
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| *index < petal_index)
                        .map(|(_, &petal)| petal)
                        .collect();
                }

                let culling_result = cull_back_area(
                    &layers[i][petal_index].area,
                    lower_or_same_layer
                        .iter()
                        .map(|petal| &petal.area)
                        .collect::<Vec<&Area>>()
                        .as_slice(),
                );
                match culling_result {
                    CullingResult::NoAreaVisible => {
                        layers[i].remove(petal_index);
                    }
                    CullingResult::Culled(area) => {
                        layers[i][petal_index].area = area;
                    }
                    CullingResult::WholeAreaVisible => {}
                }
            }
        }
    }

    {
        let mut index = layers.len();
        while let Some(next) = index.checked_sub(1) {
            index = next;

            if layers[index].is_empty() {
                layers.remove(index);
            }
        }
    }
}

#[allow(clippy::ptr_arg)]
fn shuffle_petals<R: Rng>(layers: &mut Vec<Vec<Petal>>, random: &mut R) {
    for layer in layers.iter_mut() {
        layer.shuffle(random);
    }
}

fn random_layer<R: Rng>(options: &LayerGenOptions, random: &mut R) -> Result<Vec<Petal>> {
    struct FloatPetal {
        side1: Vec<Vec2>,
        side2: Vec<Vec2>,
        rotation: f32,
        expected_size: u16,
    }

    fn new_petal<R: Rng>(options: &LayerGenOptions, random: &mut R) -> Result<FloatPetal> {
        let mirror_direction = options.mirror_direction.get(random) > 0.0;
        let mut gen_side = |mirror_direction: bool| -> Result<Vec<Vec2>> {
            petal_side(PetalOptions {
                k: options.k.get(random),
                flip: options.flip,
                mirror: mirror_direction,
                function: options.petal_function,
            })
            .context("failed to generate petal size")
        };
        let mut side1 = gen_side(mirror_direction)?;
        let mut side2 = gen_side(if options.mirror {
            !mirror_direction
        } else {
            mirror_direction
        })?;

        let size = {
            let result = options.size.get(random) - options.petal_distance_from_origin as f32;
            debug_assert!(result >= 1.0);
            result as u16
        };

        {
            let distance = normalize(
                options.petal_distance_from_origin as f32,
                0.0,
                (size + options.petal_distance_from_origin) as f32,
                0.0,
                1.0,
            );
            for point in side1.iter_mut().chain(side2.iter_mut()) {
                point.x += distance;
            }
        }

        Ok(FloatPetal {
            side1,
            side2,
            rotation: 0.0,
            expected_size: size,
        })
    }

    let mut petals = {
        let initial_angle = random.gen_range(-PI..=PI);
        match options.petal_arrangement {
            PetalArrangement::Valvate {
                max_interpetal_angle_delta,
            } => {
                debug_assert_finite!(max_interpetal_angle_delta);
                let mut petals = Vec::new();
                let mut angle = initial_angle;

                while angle < initial_angle + (PI * 2.0) {
                    let petal = new_petal(options, random).context(format!(
                        "failed to generate petal with valvate arrangement, angle: {angle}"
                    ))?;
                    angle += {
                        let mut min = f32::INFINITY;
                        let mut max = f32::NEG_INFINITY;

                        for point in petal.side1.iter().chain(petal.side2.iter()) {
                            let angle = point.to_angle();
                            min = f32::min(min, angle);
                            max = f32::max(max, angle);
                        }

                        if !min.is_finite() || !max.is_finite() {
                            bail!("failed to find petal min/max angle");
                        }

                        max - min
                    };
                    petals.push(petal);
                }

                let angle_step = angle / petals.len() as f32;
                debug_assert_finite!(angle_step);

                for (index, petal) in petals.iter_mut().enumerate() {
                    petal.rotation += initial_angle + (index as f32 * angle_step);
                    petal.rotation +=
                        random.gen_range(-max_interpetal_angle_delta..=max_interpetal_angle_delta);
                }

                petals
            }
            PetalArrangement::RadiallySymmetrical {
                max_interpetal_angle_delta,
                petals_count,
            } => {
                debug_assert_finite!(max_interpetal_angle_delta);
                let mut petals = Vec::with_capacity(petals_count as usize);

                for i in 0..petals_count {
                    let mut petal = new_petal(options, random)?;

                    petal.rotation +=
                        initial_angle + ((i as f32 / petals_count as f32) * (PI * 2.0));
                    petal.rotation +=
                        random.gen_range(-max_interpetal_angle_delta..=max_interpetal_angle_delta);

                    petals.push(petal)
                }

                petals
            }
        }
    };

    for petal in &mut petals {
        let rotation = Mat2::from_angle(petal.rotation);
        for point in petal.side1.iter_mut().chain(petal.side2.iter_mut()) {
            *point = rotation.mul_vec2(*point);
        }
    }

    let scaled_petals: Vec<Petal> = petals
        .into_iter()
        .filter_map(|petal| {
            let skeleton = {
                let scaled_side1 = scale(&petal.side1, petal.expected_size);
                let scaled_side2 = scale(&petal.side2, petal.expected_size);
                let merged = merge(&[&scaled_side1, &scaled_side2], MergeMode::ZigZag);
                interpolate(&merged)
            };
            let area = find_inner_area(&skeleton)?;
            Some(Petal::new(skeleton, area))
        })
        .collect();

    if scaled_petals.is_empty() {
        bail!("generated a layer with no petals");
    }

    Ok(scaled_petals)
}

fn petal_side(options: PetalOptions) -> Result<Vec<Vec2>> {
    let step = 0.0001;
    let angle = if options.flip { PI } else { 0.0 };
    let k = match options.function {
        PetalFunction::Sin => options.k,
        PetalFunction::Tan => options.k / 2.0,
    };

    let mut evaluated_polar_func = match options.function {
        PetalFunction::Sin => eval_polar_sin(k, step, angle, options.mirror),
        PetalFunction::Tan => eval_polar_sin(k, step, angle, options.mirror),
    };

    if evaluated_polar_func.is_empty() {
        bail!(
            "failed to generate petal side with following options: {:?}",
            options
        );
    }

    if options.flip {
        for point in &mut evaluated_polar_func {
            point.x += 1.0;
        }
    }

    Ok(evaluated_polar_func)
}

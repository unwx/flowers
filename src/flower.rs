use crate::color::{
    color_to_hsl, colorize, hsl_to_color, random_gradient, random_noise, random_palette,
    DynGradient, DynNoise, MAX_COLOR_LIGHT, MIN_COLOR_LIGHT,
};
use crate::graphics::Drawing;
use crate::math::normalize_f32;
use crate::mosaic::{random_mosaic, Mosaic};
use crate::petal::{find_petal_area, scale_and_merge_sides, side_sin, side_tan, MergeMode};
use crate::rand::RecoverableRng;
use colorgrad::Color;
use glam::{I16Vec2, Mat2, Vec2};
use rand::prelude::SliceRandom;
use rand::{Rng, RngCore};
use std::f32::consts::PI;
use std::ops::{Div, RangeInclusive};
use std::rc::Rc;

pub struct Flower {
    pub mosaic: Mosaic,
    pub layers: Vec<Layer>,
    pub background_color: Color,
    pub size: u16,
}

pub struct Layer {
    pub petals: Vec<Drawing>,
}

#[derive(Copy, Clone)]
enum PetalArrangement {
    Valvate {
        initial_angle: f32,
        max_interpetal_angle_delta: f32,
    },
    RadiallySymmetrical {
        initial_angle: f32,
        max_interpetal_angle_delta: f32,
        petals_count: u16,
    },
}

#[derive(Copy, Clone)]
struct RandomValue {
    value: f32,
    max_delta: f32,
}

impl RandomValue {
    fn from_range<R: Rng>(range: RangeInclusive<f32>, random: &mut R) -> Self {
        Self::from_min_max(*range.start(), *range.end(), random)
    }

    fn from_min_max<R: Rng>(min_value: f32, max_value: f32, random: &mut R) -> Self {
        let value = random.gen_range(min_value..=max_value);
        let max_delta = random.gen_range(0.0..=(value - min_value).min(max_value - value));

        Self { value, max_delta }
    }

    fn get<R: Rng>(self, random: &mut R) -> f32 {
        self.value + random.gen_range(-self.max_delta..=self.max_delta)
    }

    #[rustfmt::skip]
    fn clone_within_delta<R: Rng>(self, random: &mut R) -> RandomValue {
        let new_value = random.gen_range((self.value - self.max_delta)..=(self.value + self.max_delta));
        let new_delta = random.gen_range(0.0..=(self.max_delta - (new_value - self.value).abs()));

        Self {
            value: new_value,
            max_delta: new_delta,
        }
    }
}

impl Div<f32> for RandomValue {
    type Output = RandomValue;

    fn div(self, rhs: f32) -> Self::Output {
        Self {
            value: self.value / rhs,
            max_delta: self.max_delta / rhs,
        }
    }
}

#[derive(Clone)]
struct RandomGradient {
    palette: Vec<Color>,
    gradient_seed: Option<u64>,
    gradient: Option<Rc<DynGradient>>,
}

impl RandomGradient {
    fn get<R: Rng>(&self, random: &mut R) -> Rc<DynGradient> {
        self.gradient.clone().unwrap_or_else(|| {
            Rc::new(random_gradient(
                self.palette.as_slice(),
                &mut RecoverableRng::new(self.gradient_seed.unwrap_or_else(|| random.next_u64())),
            ))
        })
    }
}

#[derive(Clone)]
struct RandomNoise {
    noise_seed: Option<u64>,
    noise_internal_seed: Option<u32>,
    noise: Option<Rc<DynNoise<f64, 2>>>,
}

impl RandomNoise {
    #[rustfmt::skip]
    fn get<R: Rng>(&self, random: &mut R) -> Rc<DynNoise<f64, 2>> {
        self.noise.clone().unwrap_or_else(|| {
            Rc::new(random_noise(
                self.noise_internal_seed.unwrap_or_else(|| random.next_u32()),
                &mut RecoverableRng::new(self.noise_seed.unwrap_or_else(|| random.next_u64())),
            ))
        })
    }
}

struct LayerGenOptions<'a> {
    gradient: &'a RandomGradient,
    noise: &'a RandomNoise,

    mirror: bool,
    flip: bool,
    petal_distance_from_origin: u16,
    petal_arrangement: PetalArrangement,

    k: RandomValue,
    noise_scale: RandomValue,
    size: RandomValue,
}

struct PetalGenOptions<'a> {
    gradient: &'a DynGradient,
    noise: &'a DynNoise<f64, 2>,

    mirror: bool,
    flip: bool,

    k: f32,
    size: u16,
    noise_scale: f32,
}

const MIN_RADIUS: u16 = 16;
const MAX_RADIUS: u16 = (i16::MAX as u16 / 2) - 1;

pub fn random_flower(radius: u16, random: &mut RecoverableRng) -> Option<Flower> {
    assert!(
        radius >= MIN_RADIUS && radius <= MAX_RADIUS,
        "illegal radius '{radius}', allowed: [{MIN_RADIUS} <= size <= {MAX_RADIUS}]",
    );

    let mosaic_radius_range =
        ((radius as f32 * 0.03).round() as u16)..=((radius as f32 * 0.40).round() as u16);
    let mosaic_radius = random.gen_range(mosaic_radius_range.clone());
    let mosaic = random_mosaic(mosaic_radius, &mut random.recover())?;

    let layers_count = random.gen_range(1..=12);
    let mut layers = Vec::with_capacity(layers_count);

    let gradient = {
        let palette = random_palette(
            random.gen_range(2..=6),
            mosaic.average_color.clone(),
            random,
        );
        let gradient_seed = random.gen_option(|r| r.next_u64());
        let gradient = random.gen_option(|r| Rc::new(random_gradient(palette.as_slice(), r)));

        RandomGradient {
            palette,
            gradient_seed,
            gradient,
        }
    };
    let noise = RandomNoise {
        noise_seed: random.gen_option(|r| r.next_u64()),
        noise_internal_seed: random.gen_option(|r| r.next_u32()),
        noise: random.gen_option(|r| Rc::new(random_noise(r.next_u32(), r))),
    };

    let petal_arrangement_choice_range = 0..2;
    let petal_arrangement_choice =
        random.gen_option(|r| r.gen_range(petal_arrangement_choice_range.clone()));

    let mirror = random.gen_option(|r| r.gen_bool(0.5));
    let flip = random.gen_option(|r| r.gen_bool(0.5));

    let k_range = 1.1..=6.0;
    let k = RandomValue::from_range(k_range.clone(), random);
    let noise_scale = {
        let value_range = 0.1..=7.5;

        if noise.noise.is_some()
            || (noise.noise_seed.is_some() && noise.noise_internal_seed.is_some())
        {
            RandomValue {
                value: random.gen_range(value_range.clone()),
                max_delta: 0.0,
            }
        } else {
            RandomValue::from_range(value_range, random)
        }
    };

    let petal_distance_from_origin_range = 0..=((radius as f32 * 0.8) as u16);
    let petal_distance_from_origin = 0; // TODO random.gen_range(petal_distance_from_origin_range.clone());
    let min_petal_length = {
        let min_length = (petal_distance_from_origin as f32 * 1.1)
            .min(i16::MAX as f32)
            .min(radius as f32) as u16;
        random.gen_range(min_length..=radius)
    };

    for i in (0..layers_count).rev() {
        let gen_options = LayerGenOptions {
            gradient: &gradient,
            noise: &noise,
            mirror: mirror.unwrap_or_else(|| random.gen_bool(0.5)),
            flip: flip.unwrap_or_else(|| random.gen_bool(0.5)),
            petal_distance_from_origin,
            petal_arrangement: {
                let initial_angle = random.gen_range(-PI..=PI);
                let max_interpetal_angle_delta = {
                    let max_delta = normalize_f32(
                        k.value,
                        *k_range.start(),
                        *k_range.end(),
                        PI / 8.0,
                        PI / 4.0,
                    );
                    random.gen_range(0.0..=max_delta)
                };

                match petal_arrangement_choice
                    .unwrap_or_else(|| random.gen_range(petal_arrangement_choice_range.clone()))
                {
                    0 => PetalArrangement::Valvate {
                        initial_angle,
                        max_interpetal_angle_delta,
                    },
                    1 => PetalArrangement::RadiallySymmetrical {
                        initial_angle,
                        max_interpetal_angle_delta,
                        petals_count: {
                            let max_petals_count = normalize_f32(
                                k.value,
                                *k_range.start(),
                                *k_range.end(),
                                10.0,
                                40.0,
                            ) * normalize_f32(
                                petal_distance_from_origin as f32,
                                *petal_distance_from_origin_range.start() as f32,
                                *mosaic_radius_range.end() as f32,
                                1.0,
                                3.0,
                            );
                            random.gen_range((max_petals_count / 3.5)..=max_petals_count) as u16
                        },
                    },
                    _ => unreachable!(),
                }
            },
            k: k.clone_within_delta(random),
            noise_scale: noise_scale.clone_within_delta(random),
            size: {
                let value = normalize_f32(
                    i as f32,
                    0.0,
                    layers_count as f32 - 1.0,
                    min_petal_length as f32,
                    radius as f32,
                );
                let max_delta = ((radius as f32 - min_petal_length as f32) / (layers_count * 2) as f32)
                    .min(value - min_petal_length as f32)
                    .min(radius as f32 - value);
                let max_delta = random.gen_range(0.0..=max_delta);

                RandomValue { value, max_delta }
            },
        };

        layers.push(random_layer(&gen_options, random)?);
    }

    {
        let change = {
            let light = color_to_hsl(mosaic.average_color.clone()).lightness;
            let value = normalize_f32(0.5 - (0.5 - light).abs(), 0.0, 0.5, 0.01, 0.04);
            let value = random.gen_range(0.01..=value);

            if light < 0.5 {
                -value
            } else {
                value
            }
        };

        for (i, layer) in layers.iter_mut().enumerate().skip(1) {
            for petal in &mut layer.petals {
                for (_, color) in &mut petal.pixels {
                    let mut hsl = color_to_hsl(color.clone());
                    hsl.lightness = (hsl.lightness + (change * i as f32))
                        .clamp(MIN_COLOR_LIGHT, MAX_COLOR_LIGHT);
                    *color = hsl_to_color(hsl)
                }
            }
        }
    }

    let background_color = mosaic.background_color.clone();
    Some(Flower {
        mosaic,
        layers,
        background_color,
        size: radius * 2,
    })
}

fn random_layer<R: Rng>(options: &LayerGenOptions, random: &mut R) -> Option<Layer> {
    fn new_petal<R: Rng>(options: &LayerGenOptions, random: &mut R) -> Option<Drawing> {
        random_petal(
            &PetalGenOptions {
                gradient: options.gradient.get(random).as_ref(),
                noise: options.noise.get(random).as_ref(),
                mirror: options.mirror,
                flip: options.flip,
                k: options.k.get(random),
                size: options.size.get(random).round().max(1.0) as u16,
                noise_scale: options.noise_scale.get(random),
            },
            random,
        )
        .map(|mut petal| {
            petal += I16Vec2::new(0, options.petal_distance_from_origin as i16);
            petal
        })
    }

    let mut petals: Vec<Drawing> = match options.petal_arrangement {
        PetalArrangement::Valvate {
            initial_angle,
            max_interpetal_angle_delta,
        } => {
            struct PetalWithAngles {
                petal: Drawing,
                min_angle: f32,
                max_angle: f32,
            }

            let mut current_angle = initial_angle;
            let to_angle = initial_angle + (PI * 2.0);
            let mut petal_with_angles_vec = Vec::new();

            while current_angle < to_angle {
                let petal = new_petal(&options, random)?;
                let min_angle = petal
                    .skeleton
                    .iter()
                    .min_by_key(|point| point.x)
                    .map(|point| point.as_vec2().to_angle())?;
                let max_angle = petal
                    .skeleton
                    .iter()
                    .max_by_key(|point| point.x)
                    .map(|point| point.as_vec2().to_angle())?;

                petal_with_angles_vec.push(PetalWithAngles {
                    petal,
                    min_angle,
                    max_angle,
                });
                current_angle += min_angle - max_angle;
            }

            let angle_leveler = (current_angle - to_angle) / petal_with_angles_vec.len() as f32;
            let mut last_angle = initial_angle;

            for petal_with_angles in &mut petal_with_angles_vec {
                let angle = (last_angle - angle_leveler) + (petal_with_angles.max_angle - petal_with_angles.min_angle);
                last_angle = angle;

                petal_with_angles.petal *= Mat2::from_angle(
                    angle + random.gen_range(-max_interpetal_angle_delta..=max_interpetal_angle_delta),
                );
            }

            petal_with_angles_vec
                .into_iter()
                .map(|item| item.petal)
                .collect()
        }
        PetalArrangement::RadiallySymmetrical {
            initial_angle,
            max_interpetal_angle_delta,
            petals_count,
        } => {
            let mut petals = Vec::with_capacity(petals_count as usize);
            for i in 0..petals_count {
                let mut petal = new_petal(&options, random)?;

                let mut angle = initial_angle + ((i as f32 / petals_count as f32) * (PI * 2.0));
                angle += random.gen_range(-max_interpetal_angle_delta..=max_interpetal_angle_delta);

                petal *= Mat2::from_angle(angle);
                petals.push(petal)
            }

            petals
        }
    };

    petals.shuffle(random);
    petals.shrink_to_fit();
    Some(Layer { petals })
}

fn random_petal<R: Rng>(options: &PetalGenOptions, random: &mut R) -> Option<Drawing> {
    let skeleton = {
        let (mirror1, mirror2) = if options.mirror {
            (false, true)
        } else {
            let base = random.gen_bool(0.5);
            (base, base)
        };
        let angle = if options.flip { PI } else { 0.0 };

        fn side<R: Rng>(options: &PetalGenOptions, angle: f32, mirror: bool, random: &mut R) -> Vec<Vec2> {
            let step = normalize_f32(options.size as f32, 0.0, MAX_RADIUS as f32, 0.001, 0.00001);
            if random.gen_bool(0.5) {
                side_sin(options.k, step, angle, mirror)
            } else {
                side_tan(options.k / 2.0, step, angle, mirror)
            }
        }

        let side1 = side(options, angle, mirror1, random);
        let side2 = side(options, angle, mirror2, random);

        if side1.is_empty() || side2.is_empty() {
            return None;
        }

        Some(scale_and_merge_sides(
            side1.as_slice(),
            side2.as_slice(),
            options.size,
            MergeMode::SideWithSide,
        ))
    }?;
    let area = find_petal_area(skeleton.as_slice());

    let (pixels, average_color) = colorize(
        area.as_slice(),
        options.gradient,
        options.noise,
        options.noise_scale,
    )?;

    Some(Drawing {
        skeleton,
        pixels,
        average_color,
    })
}

use crate::color::{
    color_to_hsl, colorize, hsl_to_color, random_gradient, random_noise, random_palette,
    DynGradient, DynNoise, MAX_COLOR_LIGHT, MIN_COLOR_LIGHT,
};
use crate::graphics::Drawing;
use crate::math::{find_nearest_f32, normalize_f32, wrap_radians};
use crate::mosaic::{random_mosaic, Mosaic};
use crate::petal::{
    find_petal_area, find_visible_back_area, scale_and_merge_sides, side_sin, side_tan, MergeMode,
};
use crate::rand::RecoverableRng;
use colorgrad::Color;
use glam::{I16Vec2, Mat2, Vec2};
use palette::num::MinMax;
use rand::prelude::SliceRandom;
use rand::{Rng, RngCore};
use std::f32::consts::PI;
use std::ops::{Div, RangeInclusive};
use std::rc::Rc;
use std::time::Instant;

pub struct Flower {
    pub mosaic: Mosaic,
    pub petals: Vec<Drawing>,
    pub background_color: Color,
    pub size: u16,
}

#[derive(Copy, Clone)]
enum PetalFunction {
    Sin,
    Tan,
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
    gradient: Option<Rc<DynGradient>>,
}

impl RandomGradient {
    fn get<R: Rng>(&self, random: &mut R) -> Rc<DynGradient> {
        self.gradient.clone().unwrap_or_else(|| {
            Rc::new(random_gradient(
                self.palette.as_slice(),
                &mut RecoverableRng::new(random.next_u64()),
            ))
        })
    }
}

#[derive(Clone)]
struct RandomNoise {
    noise_seed: Option<u64>,
    noise: Option<Rc<DynNoise<f64, 2>>>,
}

impl RandomNoise {
    fn get<R: Rng>(&self, random: &mut R) -> Rc<DynNoise<f64, 2>> {
        self.noise.clone().unwrap_or_else(|| {
            Rc::new(random_noise(
                random.next_u32(),
                &mut RecoverableRng::new(self.noise_seed.unwrap_or_else(|| random.next_u64())),
            ))
        })
    }
}

#[derive(Clone)]
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

#[derive(Copy, Clone)]
struct PetalOptions {
    k: f32,
    flip: bool,
    mirror: bool,
    mirror_direction: bool,
    function: PetalFunction,
}

struct Petal {
    skeleton: Vec<I16Vec2>,
    area: Vec<(I16Vec2, I16Vec2)>,
    left_side_angle: f32,
    center_angle: f32,
    right_side_angle: f32,
}

impl Petal {
    pub fn new(
        skeleton: Vec<I16Vec2>,
        area: Vec<(I16Vec2, I16Vec2)>,
        left_side_angle: f32,
        center_angle: f32,
        right_side_angle: f32,
    ) -> Self {
        let mut petal = Self {
            skeleton,
            area,
            left_side_angle: wrap_radians(left_side_angle),
            center_angle: wrap_radians(center_angle),
            right_side_angle: wrap_radians(right_side_angle),
        };

        petal
    }
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
        let gradient = random.gen_option(|r| Rc::new(random_gradient(palette.as_slice(), r)));

        RandomGradient { palette, gradient }
    };
    let noise = RandomNoise {
        noise_seed: random.gen_option(|r| r.next_u64()),
        noise: random.gen_option(|r| Rc::new(random_noise(r.next_u32(), r))),
    };

    let petal_arrangement_choice_range = 0..2;
    let petal_arrangement_choice =
        random.gen_option(|r| r.gen_range(petal_arrangement_choice_range.clone()));

    let flip = random.gen_bool(0.5);
    let mirror = random.gen_bool(0.5);
    let mirror_direction = RandomValue {
        value: 0.0,
        max_delta: if random.gen_bool(0.5) { 1.0 } else { 0.0 },
    };
    let petal_function = if random.gen_bool(0.5) {
        PetalFunction::Sin
    } else {
        PetalFunction::Tan
    };

    let k_range = 1.1..=6.0;
    let k = RandomValue::from_range(k_range.clone(), random);
    let noise_scale = random.gen_range(0.1..=10.0);

    let petal_distance_from_origin_range = 0..=((mosaic_radius as f32 * 0.8) as u16);
    let petal_distance_from_origin = random.gen_range(petal_distance_from_origin_range.clone());
    let min_petal_length = {
        let min_length = (petal_distance_from_origin as f32 * 1.1).min(radius as f32) as u16;
        random.gen_range(min_length..=radius)
    };

    for i in (0..layers_count).rev() {
        let gen_options = LayerGenOptions {
            flip,
            mirror,
            mirror_direction,
            petal_function,
            petal_arrangement: {
                let initial_angle = random.gen_range(-PI..=PI);
                let max_interpetal_angle_delta = random.gen_range(0.0..=(PI / 4.0));

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
            petal_distance_from_origin,
            k,
            size: {
                let value = normalize_f32(
                    i as f32 + 1.0,
                    1.0,
                    layers_count as f32,
                    min_petal_length as f32,
                    radius as f32,
                );
                let max_delta = ((radius as f32 - min_petal_length as f32)
                    / (layers_count * 2).max(1) as f32)
                    .min(value - min_petal_length as f32)
                    .min(radius as f32 - value);
                let max_delta = random.gen_range(0.0..=max_delta);

                RandomValue { value, max_delta }
            },
        };

        layers.push(random_layer(&gen_options, random)?);
    }

    optimize_layers_and_shuffle_petals(&mut layers, random);
    let petals = draw_layers(
        layers,
        mosaic.average_color.clone(),
        &gradient,
        &noise,
        noise_scale,
        random,
    )?;
    let background_color = mosaic.background_color.clone();

    Some(Flower {
        mosaic,
        petals,
        background_color,
        size: radius * 2,
    })
}

fn draw_layers<R: Rng>(
    layers: Vec<Vec<Petal>>,
    average_color: Color,
    gradient: &RandomGradient,
    noise: &RandomNoise,
    noise_scale: f32,
    random: &mut R,
) -> Option<Vec<Drawing>> {
    let light_change = {
        let light = color_to_hsl(average_color).lightness;
        let value = normalize_f32(0.5 - (0.5 - light).abs(), 0.0, 0.5, 0.01, 0.04);
        let value = random.gen_range(0.01..=value);

        if light < 0.5 {
            value
        } else {
            -value
        }
    };

    let mut drawings = Vec::with_capacity(layers.len());
    for (layer_index, layer) in layers.into_iter().enumerate() {
        for petal in layer {
            let (mut pixels, color) = colorize(
                petal.area.as_slice(),
                gradient.get(random).as_ref(),
                noise.get(random).as_ref(),
                noise_scale,
            )?;

            for (_, color) in &mut pixels {
                let mut hsl = color_to_hsl(color.clone());
                hsl.lightness = (hsl.lightness + (light_change * layer_index as f32))
                    .clamp(MIN_COLOR_LIGHT, MAX_COLOR_LIGHT);
                *color = hsl_to_color(hsl)
            }

            drawings.push(Drawing {
                skeleton: petal.skeleton,
                pixels,
                average_color: color,
            })
        }
    }

    Some(drawings)
}

fn optimize_layers_and_shuffle_petals<R: Rng>(layers: &mut Vec<Vec<Petal>>, random: &mut R) {
    // The biggest petals are first.

    let mut orders = Vec::with_capacity(layers.len());
    for layer in layers.iter() {
        let mut order = Vec::with_capacity(layer.len());
        for i in 0..layer.len() {
            order.push(i);
        }

        order.shuffle(random);
        orders.push(order);
    }

    let (left_angle_sorted_layers, right_angle_sorted_layers) = {
        fn sort_layers<F>(layers: &mut Vec<Vec<Petal>>, to_angle: F) -> Vec<Vec<(usize, f32)>>
        where
            F: Fn(&Petal) -> f32,
        {
            let mut sorted_layers = Vec::with_capacity(layers.len());
            for layer in layers.iter() {
                let mut sorted = Vec::with_capacity(layer.len());
                for (index, petal) in layer.iter().enumerate() {
                    sorted.push((index, to_angle(petal)));
                }

                sorted.sort_unstable_by(|p1, p2| p1.1.total_cmp(&p2.1));
                sorted_layers.push(sorted);
            }

            sorted_layers
        }

        (
            sort_layers(layers, |petal| petal.left_side_angle),
            sort_layers(layers, |petal| petal.right_side_angle),
        )
    };

    fn optimize_back_area(
        back: &[(I16Vec2, I16Vec2)],
        front_areas: &[&[(I16Vec2, I16Vec2)]],
    ) -> Vec<(I16Vec2, I16Vec2)> {
        if front_areas.is_empty() {
            return back.to_vec();
        }

        let mut visible_area = find_visible_back_area(back, front_areas[0]);
        for front in front_areas.iter().skip(1) {
            visible_area = find_visible_back_area(visible_area.as_slice(), front);
        }

        visible_area
    }

    fn find_overlapping_petals(
        petal: &Petal,
        petals: &Vec<Petal>,
        left_sorted_petals: &Vec<(usize, f32)>,
        right_sorted_petals: &Vec<(usize, f32)>,
    ) -> Vec<usize> {
        if petals.is_empty() {
            return vec![];
        }

        let mut left_index = find_nearest_f32(
            left_sorted_petals.as_slice(),
            petal.left_side_angle,
            |petal| petal.1,
        )
        .unwrap_or(0);
        let mut right_index = find_nearest_f32(
            right_sorted_petals.as_slice(),
            petal.right_side_angle,
            |petal| petal.1,
        )
        .unwrap_or(right_sorted_petals.len() - 1);

        let adjust_petal_index =
            |index: usize, direction: isize, sorted_petals: &Vec<(usize, f32)>| -> usize {
                let overlaps = |other_petal: &Petal| -> bool {
                    if petal.left_side_angle <= petal.right_side_angle {
                        petal.left_side_angle <= other_petal.right_side_angle
                            && other_petal.left_side_angle <= petal.right_side_angle
                    } else {
                        petal.left_side_angle <= other_petal.right_side_angle
                            || other_petal.left_side_angle <= petal.right_side_angle
                    }
                    // true
                };
                let wrap_index = |index: usize, offset: isize| -> usize {
                    ((index as isize) + offset).rem_euclid(petals.len() as isize) as usize
                };

                let mut adjusted_index = wrap_index(index, 0);
                for _ in 0..petals.len() {
                    let wrapped_index = wrap_index(adjusted_index, direction);
                    if overlaps(&petals[sorted_petals[wrapped_index].0]) {
                        adjusted_index = wrapped_index;
                    } else {
                        break;
                    }
                }

                if !overlaps(&petals[sorted_petals[adjusted_index].0]) {
                    for _ in 0..petals.len() {
                        let wrapped_index = wrap_index(adjusted_index, -direction);
                        if overlaps(&petals[sorted_petals[wrapped_index].0]) {
                            adjusted_index = wrapped_index;
                            break;
                        }
                    }
                }

                adjusted_index
            };

        left_index = adjust_petal_index(left_index, -1, left_sorted_petals);
        right_index = adjust_petal_index(right_index, 1, right_sorted_petals);

        // let from_left_petal_index = left_sorted_petals[left_index].0;
        // let to_right_petal_index = {
        //     let index = right_sorted_petals[right_index].0;
        //     if index < from_left_petal_index {
        //         index + petals.len()
        //     } else {
        //         index
        //     }
        // }; TODO fix angles
        let from_left_petal_index = 0;
        let to_right_petal_index = petals.len() - 1;

        let mut overlapping_petals =
            Vec::with_capacity((to_right_petal_index - from_left_petal_index) + 1);
        for i in from_left_petal_index..=to_right_petal_index {
            overlapping_petals.push(i.rem_euclid(petals.len()));
        }

        overlapping_petals
    }

    for i in 0..layers.len() {
        for y in i..layers.len() {
            for petal_index in 0..layers[i].len() {
                let petal = &layers[i][petal_index];
                let mut overlapping_petals = find_overlapping_petals(
                    petal,
                    &layers[y],
                    &left_angle_sorted_layers[y],
                    &right_angle_sorted_layers[y],
                );

                if i == y {
                    let petal_order = orders[i][petal_index];
                    overlapping_petals = overlapping_petals
                        .iter()
                        .filter(|&&other_petal_index| orders[y][other_petal_index] > petal_order)
                        .map(|&other_petal_index| other_petal_index)
                        .collect();
                }

                let optimized_area = optimize_back_area(
                    petal.area.as_slice(),
                    overlapping_petals
                        .iter()
                        .map(|&other_petal_index| layers[y][other_petal_index].area.as_slice())
                        .collect::<Vec<&[(I16Vec2, I16Vec2)]>>()
                        .as_slice(),
                );

                let petal = &mut layers[i][petal_index];
                petal.area = optimized_area;
            }
        }
    }
}

fn random_layer<R: Rng>(options: &LayerGenOptions, random: &mut R) -> Option<Vec<Petal>> {
    struct FloatPetal {
        sides: (Vec<Vec2>, Vec<Vec2>),
        left_side_angle: f32,
        center_angle: f32,
        right_side_angle: f32,
        expected_size: u16,
    }

    impl FloatPetal {
        fn shift_angles(&mut self, shift: f32) {
            self.left_side_angle += shift;
            self.center_angle += shift;
            self.right_side_angle += shift;
            self.wrap_angles();
        }

        fn wrap_angles(&mut self) {
            self.left_side_angle = wrap_radians(self.left_side_angle);
            self.center_angle = wrap_radians(self.center_angle);
            self.right_side_angle = wrap_radians(self.right_side_angle);
        }
    }

    fn new_petal<R: Rng>(options: &LayerGenOptions, random: &mut R) -> Option<FloatPetal> {
        let mut skeleton = petal_sides(PetalOptions {
            k: options.k.get(random),
            flip: options.flip,
            mirror: options.mirror,
            mirror_direction: options.mirror_direction.get(random) > 0.0,
            function: options.petal_function,
        });
        let size = (options.size.get(random) - options.petal_distance_from_origin as f32) as u16;

        {
            let distance = normalize_f32(
                options.petal_distance_from_origin as f32,
                0.0,
                (size + options.petal_distance_from_origin) as f32,
                0.0,
                1.0,
            );
            for point in skeleton.0.iter_mut().chain(skeleton.1.iter_mut()) {
                point.y += distance;
            }
        }

        let left_side_angle = skeleton
            .0
            .iter()
            .chain(skeleton.1.iter())
            .min_by(|point1, point2| point1.x.total_cmp(&point2.x))
            .map(|point| point.to_angle())?;
        let right_side_angle = skeleton
            .0
            .iter()
            .chain(skeleton.1.iter())
            .max_by(|point1, point2| point1.x.total_cmp(&point2.x))
            .map(|point| point.to_angle())?;

        Some(FloatPetal {
            sides: skeleton,
            left_side_angle: wrap_radians(left_side_angle),
            center_angle: PI / 2.0,
            right_side_angle: wrap_radians(right_side_angle),
            expected_size: size,
        })
    }

    let mut petals = match options.petal_arrangement {
        PetalArrangement::Valvate {
            initial_angle,
            max_interpetal_angle_delta,
        } => {
            let mut petals = Vec::new();
            let circle = initial_angle + (PI * 2.0);

            {
                let mut angle = initial_angle;
                while angle < circle {
                    let petal = new_petal(&options, random)?;
                    angle += {
                        if petal.right_side_angle >= petal.left_side_angle {
                            petal.right_side_angle - petal.left_side_angle
                        } else {
                            let wrapped_right_side_angle = petal.right_side_angle + (PI * 2.0);
                            if wrapped_right_side_angle < petal.left_side_angle {
                                debug_assert!(false);
                                break;
                            }

                            wrapped_right_side_angle - petal.left_side_angle
                        }
                    };
                    petals.push(petal);
                }

                angle
            };

            let angle_step = circle / petals.len() as f32;
            for (index, petal) in petals.iter_mut().enumerate() {
                let mut shift = index as f32 * angle_step;
                shift += random.gen_range(-max_interpetal_angle_delta..=max_interpetal_angle_delta);
                petal.shift_angles(shift);
            }

            petals
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

                petal.shift_angles(angle);
                petals.push(petal)
            }

            petals.shrink_to_fit();
            petals
        }
    };

    for petal in &mut petals {
        let rotation = Mat2::from_angle(petal.center_angle);
        for point in petal.sides.0.iter_mut().chain(petal.sides.1.iter_mut()) {
            *point = rotation.mul_vec2(*point);
        }
    }

    let scaled_petals = petals
        .into_iter()
        .map(|petal| {
            let skeleton = scale_and_merge_sides(
                petal.sides.0.as_slice(),
                petal.sides.1.as_slice(),
                petal.expected_size,
                MergeMode::SideWithSide,
            );
            let area = find_petal_area(skeleton.as_slice());

            Petal {
                skeleton: skeleton.clone(),
                area,
                left_side_angle: petal.left_side_angle,
                center_angle: petal.center_angle,
                right_side_angle: petal.right_side_angle,
            }
        })
        .collect();

    Some(scaled_petals)
}

fn petal_sides(options: PetalOptions) -> (Vec<Vec2>, Vec<Vec2>) {
    let (mirror1, mirror2) = {
        let direction = options.mirror_direction;

        if options.mirror {
            (direction, !direction)
        } else {
            (direction, direction)
        }
    };

    let side = |mirror: bool| -> Vec<Vec2> {
        let step = 0.0001;
        let angle = if options.flip { PI } else { 0.0 };

        match options.function {
            PetalFunction::Sin => side_sin(options.k, step, angle, mirror),
            PetalFunction::Tan => side_tan(options.k / 2.0, step, angle, mirror),
        }
    };

    (side(mirror1), side(mirror2))
}

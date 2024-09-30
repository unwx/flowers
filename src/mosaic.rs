use crate::color::{colorize, random_background_color, random_color, random_gradient, random_noise, random_palette};
use crate::graphics::Drawing;
use crate::math::normalize_f32;
use crate::petal::{find_petal_area, scale_and_merge_sides, side_sin, side_tan, MergeMode};
use glam::Vec2;
use rand::Rng;
use std::f32::consts::PI;
use std::ops::{Deref, DerefMut};
use colorgrad::Color;

pub struct Mosaic {
    drawing: Drawing,
    pub background_color: Color,
    pub size: u16
}

impl Deref for Mosaic {
    type Target = Drawing;

    fn deref(&self) -> &Self::Target {
        &self.drawing
    }
}

impl DerefMut for Mosaic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.drawing
    }
}


const MIN_RADIUS: u16 = 8;
const MAX_RADIUS: u16 = (i16::MAX as u16 / 2) - 1;

pub fn random_mosaic<R: Rng>(radius: u16, random: &mut R) -> Option<Mosaic> {
    assert!(
        radius >= MIN_RADIUS && radius <= MAX_RADIUS,
        "illegal radius '{radius}', allowed: [{MIN_RADIUS} <= size <= {MAX_RADIUS}]",
    );

    let skeleton = {
        let part1 = random_mosaic_part(random);
        let part2 = random_mosaic_part(random);

        if part1.is_empty() || part2.is_empty() {
            None
        } else {
            Some(scale_and_merge_sides(
                part1.as_slice(),
                part2.as_slice(),
                radius,
                MergeMode::SideWithOrigin,
            ))
        }
    }?;
    if skeleton.is_empty() {
        return None;
    }

    let (pixels, average_color) = {
        let area = find_petal_area(skeleton.as_slice());

        let gradient = {
            let start_color = random_color(random);
            let palette = random_palette(random.gen_range(2..=6), start_color, random);
            random_gradient(palette.as_slice(), random)
        };
        let noise = random_noise(random.next_u32(), random);

        colorize(
            area.as_slice(),
            &gradient,
            &noise,
            random.gen_range(0.1..=7.5),
        )
    }?;

    Some(Mosaic {
        drawing: Drawing {
            skeleton,
            pixels,
            average_color: average_color.clone(),
        },
        background_color: random_background_color(average_color, random),
        size: radius * 2
    })
}

fn random_mosaic_part<R: Rng>(random: &mut R) -> Vec<Vec2> {
    let mirror = random.gen_bool(0.5);
    let angle = random.gen_range(-PI..=PI);

    let k_range = 0.0001..=0.01;
    let k = random.gen_range(k_range.clone());

    fn gen_step<R: Rng>(k: f32, min_k: f32, max_k: f32, random: &mut R) -> f32 {
        let min_step = 0.001;
        let max_step = 15.0;

        random.gen_range(min_step..=normalize_f32(k, min_k, max_k, max_step, max_step / 2.0))
    }

    if random.gen_bool(0.5) {
        let step = gen_step(k, *k_range.start(), *k_range.end(), random);
        side_sin(k, step, angle, mirror)
    } else {
        let step = gen_step(k / 2.0, k_range.start() / 2.0, k_range.end() / 2.0, random);
        side_tan(k, step, angle, mirror)
    }
}

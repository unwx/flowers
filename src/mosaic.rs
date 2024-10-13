use crate::graphics::color::gradient::random_gradient;
use crate::graphics::color::theory::{random_color, random_palette};
use crate::graphics::image::Raster;
use crate::graphics::noise::theory::random_noise;
use crate::graphics::render::color_area;
use crate::math::area::find_inner_area;
use crate::math::curve::{eval_polar_sin, eval_polar_tan, merge, scale, MergeMode};
use crate::math::real::{debug_assert_finite, debug_eval_finite};
use crate::math::{interpolate, normalize};
use anyhow::{bail, Context, Result};
use glam::{I16Vec2, Vec2};
use palette::rgb::Rgb;
use rand::Rng;
use std::f32::consts::PI;
use std::ops::Deref;

#[derive(Clone)]
pub struct Mosaic {
    raster: Raster,
    palette: Vec<Rgb>,
    radius: u16,
}

impl Mosaic {
    #[must_use]
    pub fn new(raster: Raster, palette: Vec<Rgb>, radius: u16) -> Self {
        Self {
            raster,
            palette,
            radius,
        }
    }

    #[must_use]
    pub fn palette(&self) -> &Vec<Rgb> {
        &self.palette
    }

    #[must_use]
    pub fn radius(&self) -> u16 {
        self.radius
    }
}

impl Deref for Mosaic {
    type Target = Raster;

    fn deref(&self) -> &Self::Target {
        &self.raster
    }
}

pub const MIN_RADIUS: u16 = 8;
pub const MAX_RADIUS: u16 = (i16::MAX as u16 / 2) - 1;

pub fn random_mosaic<R: Rng>(radius: u16, random: &mut R) -> Result<Mosaic> {
    {
        assert!(
            (MIN_RADIUS..=MAX_RADIUS).contains(&radius),
            "invalid radius ({radius}), allowed: [{MIN_RADIUS} <= radius <= {MAX_RADIUS}]",
        );
    }

    let skeleton = {
        let parts_count = 2;
        let mut parts = Vec::with_capacity(parts_count);

        for _ in 0..parts_count {
            let part = random_mosaic_part(random).context("failed to generate mosaic skeleton")?;
            parts.push(part);
        }

        let parts: Vec<Vec<I16Vec2>> = parts
            .into_iter()
            .map(|part| scale(part.as_slice(), radius))
            .collect();
        let merged = merge(
            parts
                .iter()
                .map(|part| part.as_slice())
                .collect::<Vec<&[I16Vec2]>>()
                .as_slice(),
            MergeMode::Origin(I16Vec2::ZERO),
        );

        interpolate(&merged)
    };
    if skeleton.is_empty() {
        bail!("failed to generate mosaic skeleton: skeleton is empty");
    }

    let (pixels, palette) = {
        let area = find_inner_area(&skeleton).context("failed to find mosaic inner area")?;
        let palette = random_palette(random.gen_range(2..=6), random_color(random), random);
        let gradient = random_gradient(&palette, random);
        let noise = random_noise(random.next_u32(), random);

        let pixels = color_area(&area, &gradient, &noise, random.gen_range(0.1..=7.5));
        (pixels, palette)
    };
    let raster = Raster::new(skeleton, pixels);
    Ok(Mosaic::new(raster, palette, radius))
}

fn random_mosaic_part<R: Rng>(random: &mut R) -> Result<Vec<Vec2>> {
    let mirror = random.gen_bool(0.5);
    let angle = random.gen_range(-PI..=PI);

    let k_range = 0.0001..=0.01;
    let k = random.gen_range(k_range.clone());
    debug_assert_finite!(angle, k);

    fn gen_step<R: Rng>(k: f32, min_k: f32, max_k: f32, random: &mut R) -> f32 {
        let min_step = 0.001;
        let max_step = 10.0;
        let max_step = normalize(k, min_k, max_k, max_step, max_step / 2.0);
        debug_eval_finite!(random.gen_range(min_step..=max_step))
    }

    let (part, k, step, func_name) = if random.gen_bool(0.5) {
        let step = gen_step(k, *k_range.start(), *k_range.end(), random);
        (eval_polar_sin(k, step, angle, mirror), k, step, "sin")
    } else {
        let k = k / 2.0;
        let step = gen_step(k, *k_range.start() / 2.0, *k_range.end() / 2.0, random);
        (eval_polar_tan(k, step, angle, mirror), k, step, "tan")
    };

    if part.is_empty() {
        bail!(
            "failed to generate mosaic part with [k: {k}, step: {step}]: \
            empty visualization result from the '{func_name}' polar function"
        );
    }

    Ok(part)
}

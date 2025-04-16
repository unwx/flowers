use crate::math::definition::{Point, Scalar};
use crate::math::polar::eval_polar_sin;
use crate::math::remap_bias;
use crate::util::range::is_range_within_another_ref;
use crate::Random;
use anyhow::{bail, Result};
use rand::{Rng, SeedableRng};
use std::f32::consts::PI;
use std::ops::RangeInclusive;

pub trait CurveFactory {
    fn curve(&self, seed: u64) -> Result<Vec<Point>>;
}


#[derive(Debug, Clone)]
pub struct PolarCurveFactory {
    k: RangeInclusive<f32>,
    sharpness: RangeInclusive<f32>,
    angle: RangeInclusive<f32>,
    mirror: RangeInclusive<f32>,
}

impl PolarCurveFactory {
    pub fn try_new(
        k: RangeInclusive<f32>,
        sharpness: RangeInclusive<f32>,
        angle: RangeInclusive<f32>,
        mirror: RangeInclusive<f32>,
    ) -> Result<Self> {
        if k.is_empty() {
            bail!("'k' cannot be empty");
        }
        if sharpness.is_empty() {
            bail!("'sharpness' cannot be empty");
        }
        if angle.is_empty() {
            bail!("'angle' cannot be empty");
        }
        if mirror.is_empty() {
            bail!("'mirror' cannot be empty");
        }

        if !is_range_within_another_ref(&k, &Self::k_constraint()) {
            bail!("'k' must be within [{:?}] range", Self::k_constraint());
        }
        if !is_range_within_another_ref(&sharpness, &(0.0..=1.0)) {
            bail!("'sharpness' must be within [0.0..=1.0] range");
        }

        Ok(Self {
            k,
            sharpness,
            angle,
            mirror,
        })
    }

    pub fn new_random<R: Rng>(random: &mut R) -> Self {
        let mut simple_random_range = |bounds: RangeInclusive<f32>| -> RangeInclusive<f32> {
            let start = random.gen_range(bounds.clone());
            let end = random.gen_range(bounds);
            RangeInclusive::new(start.min(end), end.max(start))
        };

        Self::try_new(
            simple_random_range(Self::k_constraint()),
            simple_random_range(0.0..=1.0),
            -PI..=PI,
            simple_random_range(-1.0..=1.0),
        )
        .expect("failed to create a random PolarCurveFactory")
    }


    pub fn k(&self) -> RangeInclusive<f32> {
        self.k.clone()
    }

    pub fn sharpness(&self) -> RangeInclusive<f32> {
        self.sharpness.clone()
    }

    pub fn angle(&self) -> RangeInclusive<f32> {
        self.angle.clone()
    }

    pub fn mirror(&self) -> RangeInclusive<f32> {
        self.mirror.clone()
    }


    pub const fn k_constraint() -> RangeInclusive<f32> {
        0.001..=0.01
    }
}

impl CurveFactory for PolarCurveFactory {
    fn curve(&self, seed: u64) -> Result<Vec<Point>> {
        let mut random = Random::seed_from_u64(seed);

        let k = random.gen_range(self.k());
        let sharpness = random.gen_range(self.sharpness());
        let angle = random.gen_range(self.angle());
        let mirror = random.gen_range(self.mirror()) > 0.0;

        let smooth_curve = eval_polar_sin(k, 0.001, angle, mirror);
        let mut sharp_curve = Vec::new();

        let mut index = 0.0;
        let step = {
            let len = smooth_curve.len() as Scalar;
            let visible_percent = remap_bias(sharpness, 0.0, 1.0, 1.0, 0.0003, 8.0);
            let visible_points = len * visible_percent;
            (len / visible_points).max(1.0)
        };

        while index < smooth_curve.len() as Scalar {
            sharp_curve.push(smooth_curve[index as usize]);
            index += step;
        }

        sharp_curve.shrink_to_fit();
        Ok(sharp_curve)
    }
}

use crate::math::curve::{merge, MergeMode};
use crate::math::definition::Point;
use crate::math::polar::{eval_polar_sin, eval_polar_tan};
use crate::math::remap;
use crate::util::range::is_range_within_another_ref;
use crate::Random;
use anyhow::Result;
use anyhow::{bail, Context};
use glam::Vec2;
use rand::{Rng, SeedableRng};
use std::ops::RangeInclusive;

pub trait PetalFactory {
    fn petal(&self, seed: u64) -> Result<Vec<Point>>;
}


#[derive(Debug, Clone)]
pub struct PolarPetalFactory {
    width: RangeInclusive<f32>,
    sharp: bool,
}

impl PolarPetalFactory {
    pub fn try_new(width: RangeInclusive<f32>, sharp: bool) -> Result<Self> {
        if width.is_empty() {
            bail!("'width' cannot be empty");
        }
        if !is_range_within_another_ref(&width, &(0.0..=1.0)) {
            bail!("'width' must be within [0.0..=1.0] range");
        }

        Ok(Self { width, sharp })
    }

    pub fn width(&self) -> RangeInclusive<f32> {
        self.width.clone()
    }

    pub fn sharp(&self) -> bool {
        self.sharp
    }


    fn petal_side<R: Rng>(&self, mirror: bool, random: &mut R) -> Result<Vec<Vec2>> {
        let sharp = self.sharp;
        let k = {
            let width = random.gen_range(self.width());
            let k = remap(width, 0.0, 1.0, 3.5, 1.1);

            if sharp {
                k / 2.0
            } else {
                k
            }
        };

        let side = if sharp {
            eval_polar_tan(k, 0.001, 0.0, mirror)
        } else {
            eval_polar_sin(k, 0.001, 0.0, mirror)
        };

        if side.is_empty() {
            bail!(
                "failed to generate a petal side: the resulting side is empty. \
                [k: {k}, sharp: {sharp}, mirror: {mirror}]"
            );
        }

        Ok(side)
    }
}

impl PetalFactory for PolarPetalFactory {
    fn petal(&self, seed: u64) -> Result<Vec<Point>> {
        let mut random = Random::seed_from_u64(seed);

        fn petal_sides<R: Rng>(
            factory: &PolarPetalFactory,
            random: &mut R,
        ) -> Result<(Vec<Point>, Vec<Point>)> {
            Ok((
                factory.petal_side(false, random)?,
                factory.petal_side(true, random)?,
            ))
        }

        let (first, second) = petal_sides(self, &mut random)
            .with_context(|| format!("failed to generate a polar petal. [seed: {seed}]"))?;

        let petal = merge(vec![first, second], MergeMode::ZigZag);
        Ok(petal)
    }
}

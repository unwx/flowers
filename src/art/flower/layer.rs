use crate::art::flower::petal::PetalFactory;
use crate::math::definition::Point;
use crate::util::range::is_range_within_another_ref;
use crate::Random;
use anyhow::Result;
use anyhow::{bail, Context};
use glam::Mat2;
use rand::{Rng, RngCore, SeedableRng};
use std::f32::consts::PI;
use std::mem;
use std::ops::RangeInclusive;

pub trait LayerFactory {
    fn layer(&self, petal_factory: &dyn PetalFactory, seed: u64) -> Result<Vec<Vec<Point>>>;
}


#[derive(Debug, Clone)]
pub struct ValvateLayerFactory {
    petal_overlap_percent: RangeInclusive<f32>,
    interpetal_angle_delta: RangeInclusive<f32>,
}

impl ValvateLayerFactory {
    pub fn try_new(
        petal_overlap_percent: RangeInclusive<f32>,
        interpetal_angle_delta: RangeInclusive<f32>,
    ) -> Result<Self> {
        if petal_overlap_percent.is_empty() {
            bail!("'petal_overlap_percent' cannot be empty");
        }
        if interpetal_angle_delta.is_empty() {
            bail!("'interpetal_angle_delta' cannot be empty");
        }
        if !is_range_within_another_ref(&petal_overlap_percent, &(0.0..=0.9)) {
            bail!("'petal_overlap_percent' must be within [0.0..=0.9] range");
        }

        Ok(Self {
            petal_overlap_percent,
            interpetal_angle_delta,
        })
    }

    pub fn petal_overlap_percent(&self) -> RangeInclusive<f32> {
        self.petal_overlap_percent.clone()
    }

    pub fn interpetal_angle_delta(&self) -> RangeInclusive<f32> {
        self.interpetal_angle_delta.clone()
    }
}

impl LayerFactory for ValvateLayerFactory {
    fn layer(&self, petal_factory: &dyn PetalFactory, seed: u64) -> Result<Vec<Vec<Point>>> {
        let mut random = Random::seed_from_u64(seed);
        let mut petals = Vec::new();

        {
            let initial_angle = random.gen_range(-PI..=PI);
            let petal_overlap_percent = random.gen_range(self.petal_overlap_percent());
            let mut angle = initial_angle;

            while angle < initial_angle + (PI * 2.0) {
                let petal = petal_factory
                    .petal(random.next_u64())
                    .context("failed to generate a valvate petal layer")?;

                if petal.is_empty() {
                    bail!("failed to generate a valvate petal layer: petal is empty");
                }


                let (min_angle, max_angle) = {
                    let mut min_angle = f32::INFINITY;
                    let mut max_angle = f32::NEG_INFINITY;

                    for &point in &petal {
                        if point == Point::ZERO {
                            continue;
                        }

                        let angle = point.to_angle();
                        min_angle = min_angle.min(angle);
                        max_angle = max_angle.max(angle);
                    }

                    if !min_angle.is_finite() || !max_angle.is_finite() {
                        bail!(
                            "failed to generate a valvate petal layer: \
                            failed to find petal angles. [petal_len: {}]",
                            petal.len()
                        );
                    }
                    if max_angle - min_angle > PI {
                        max_angle -= PI * 2.0;
                        mem::swap(&mut max_angle, &mut min_angle);
                    }

                    (min_angle, max_angle)
                };

                {
                    let rotation = {
                        let delta = random.gen_range(self.interpetal_angle_delta());
                        delta + (angle - initial_angle)
                    };
                    petals.push((petal, rotation));
                }

                angle += {
                    let occupation = max_angle - min_angle;
                    occupation * (1.0 - petal_overlap_percent)
                };
            }
        }

        for (petal, rotation) in &mut petals {
            let matrix = Mat2::from_angle(*rotation);
            for point in petal {
                *point = matrix.mul_vec2(*point);
            }
        }

        Ok(petals.into_iter().map(|(petal, _)| petal).collect())
    }
}

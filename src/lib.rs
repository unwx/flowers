#![forbid(unsafe_code)]

use crate::art::flower::FlowerFactory;
use crate::art::mosaic::{Mosaic, MosaicFactory};
use crate::constraint::{MAX_MOSAIC_RADIUS, MIN_MOSAIC_RADIUS};
use crate::render::{draw_flower, draw_mosaic};
use ::anyhow::Result;
use anyhow::Context;
use image::RgbaImage;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

pub mod art;
pub mod constraint;
pub mod math;
pub mod util;

mod color;
mod noise;
mod render;

type Random = StdRng;

pub fn random_mosaic_from_seed(
    seed: u64,
    mosaic_radius: u16,
    image_size: u16,
) -> Result<RgbaImage> {
    let mosaic = random_mosaic(seed, mosaic_radius)?;
    let mut image = RgbaImage::new(image_size as u32, image_size as u32);

    draw_mosaic(&mosaic, &mut image);
    Ok(image)
}

pub fn random_flower_from_seed(
    seed: u64,
    flower_radius: u16,
    image_size: u16,
) -> Result<RgbaImage> {
    let mut random = Random::seed_from_u64(seed);

    let mosaic = {
        let mosaic_radius_percent = random.gen_range(0.25..=0.45);
        let mut mosaic_radius = ((flower_radius as f32) * mosaic_radius_percent) as u16;
        mosaic_radius = mosaic_radius.clamp(MIN_MOSAIC_RADIUS, MAX_MOSAIC_RADIUS);

        random_mosaic(seed, mosaic_radius).with_context(|| {
            format!("failed to generate a flower. [seed: {seed}, radius: {flower_radius}]")
        })?
    };

    let flower = FlowerFactory::new_random(&mut random)
        .random_flower(mosaic, flower_radius, &mut random)
        .with_context(|| {
            format!("failed to generate a flower. [seed: {seed}, radius: {flower_radius}]")
        })?;

    let mut image = RgbaImage::new(image_size as u32, image_size as u32);
    draw_flower(&flower, &mut image);
    Ok(image)
}


fn random_mosaic(seed: u64, radius: u16) -> Result<Mosaic> {
    let mut random = Random::seed_from_u64(seed);
    MosaicFactory::new_random(&mut random)
        .random_mosaic(radius, &mut random)
        .with_context(|| format!("failed to generate a mosaic. [seed: {seed}, radius: {radius}]"))
}


#[cfg(test)]
mod tests {
    use crate::{random_flower_from_seed, random_mosaic_from_seed};
    use rand::prelude::StdRng;
    use rand::{RngCore, SeedableRng};
    use std::fs;

    #[test]
    fn mosaic() {
        let path = "dev/mosaic";
        fs::create_dir_all(path).unwrap();

        for _ in 0..10 {
            let radius = 1250;
            let image_size = (radius as f32 * 2.0 * 1.2) as u16;
            let seed = StdRng::from_entropy().next_u64();

            println!("Generating a mosaic, seed: {seed}");
            let result = random_mosaic_from_seed(seed, radius, image_size);

            match result {
                Ok(image) => image.save(format!("{path}/{seed}.png")).unwrap(),
                Err(e) => println!("{:?}", e),
            }
        }
    }

    #[test]
    fn flower() {
        let path = "dev/flower";
        fs::create_dir_all(path).unwrap();

        for _ in 0..10 {
            let radius = 1250;
            let image_size = (radius as f32 * 2.0 * 1.2) as u16;
            let seed = StdRng::from_entropy().next_u64();

            println!("Generating a flower, seed: {seed}");
            let result = random_flower_from_seed(seed, radius, image_size);

            match result {
                Ok(image) => image.save(format!("{path}/{seed}.png")).unwrap(),
                Err(e) => println!("{:?}", e),
            }
        }
    }
}

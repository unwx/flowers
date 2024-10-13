use crate::flower::random_flower;
use crate::graphics::image::{draw_flower, draw_mosaic};
use crate::mosaic::random_mosaic;
use crate::rand::RestorableRng;
use ::anyhow::Result;
use ::rand::prelude::StdRng;
use ::rand::Rng;
use anyhow::Context;
use image::RgbImage;
use rand_core::SeedableRng;

pub(crate) mod common;
pub(crate) mod flower;
pub(crate) mod graphics;
pub(crate) mod math;
pub(crate) mod mosaic;
pub(crate) mod rand;

pub fn gen_mosaic(radius: u16, blank_image_space_percent: f32) -> Result<(RgbImage, u64)> {
    let seed = random_seed();
    gen_mosaic_from_seed(seed, radius, blank_image_space_percent).map(|image| (image, seed))
}

pub fn gen_mosaic_from_seed(
    seed: u64,
    radius: u16,
    blank_image_space_percent: f32,
) -> Result<RgbImage> {
    assert!(
        (mosaic::MIN_RADIUS..=mosaic::MAX_RADIUS).contains(&radius),
        "invalid radius ({radius}), allowed: [{} <= radius <= {}]",
        mosaic::MIN_RADIUS,
        mosaic::MAX_RADIUS
    );
    assert!(
        (0.0..=1.0).contains(&blank_image_space_percent),
        "invalid blank_image_space_percent({blank_image_space_percent}), allowed: [0.0 <= blank_image_space_percent <= 1.0]"
    );

    let mut random = RestorableRng::new(seed);
    let mosaic = random_mosaic(radius, &mut random).context("failed to generate mosaic")?;
    let image = draw_mosaic(&mosaic, blank_image_space_percent).context("failed to draw mosaic")?;
    Ok(image)
}

pub fn gen_flower(radius: u16, blank_image_space_percent: f32) -> Result<(RgbImage, u64)> {
    let seed = random_seed();
    gen_flower_from_seed(seed, radius, blank_image_space_percent).map(|image| (image, seed))
}

pub fn gen_flower_from_seed(
    seed: u64,
    radius: u16,
    blank_image_space_percent: f32,
) -> Result<RgbImage> {
    assert!(
        (flower::MIN_RADIUS..=flower::MAX_RADIUS).contains(&radius),
        "invalid radius ({radius}), allowed: [{} <= radius <= {}]",
        flower::MIN_RADIUS,
        flower::MAX_RADIUS
    );
    assert!(
        (0.0..=1.0).contains(&blank_image_space_percent),
        "invalid blank_image_space_percent({blank_image_space_percent}), allowed: [0.0 <= blank_image_space_percent <= 1.0]"
    );

    let mut random = RestorableRng::new(seed);
    let flower = random_flower(radius, &mut random).context("failed to generate flower")?;
    let image = draw_flower(&flower, blank_image_space_percent).context("failed to draw flower")?;
    Ok(image)
}

fn random_seed() -> u64 {
    StdRng::from_entropy().gen_range(u64::MIN..=u64::MAX)
}

#[cfg(test)]
mod tests {
    use crate::{gen_flower_from_seed, gen_mosaic_from_seed, random_seed};
    use std::fs;

    #[test]
    fn mosaic() {
        fs::create_dir_all("dev/mosaic").unwrap();

        for _ in 0..10 {
            let radius = 1250;
            let blank_image_space_percent = 0.1;
            let seed = random_seed();

            println!("Generating mosaic, seed: {seed}");
            let result = gen_mosaic_from_seed(seed, radius, blank_image_space_percent);

            match result {
                Ok(image) => image.save(format!("dev/mosaic/{seed}.png")).unwrap(),
                Err(e) => println!("failed to generate mosaic: {:?}", e),
            }
        }
    }

    #[test]
    fn flower() {
        fs::create_dir_all("dev/flower").unwrap();

        for _ in 0..10 {
            let radius = 1250;
            let blank_image_space_percent = 0.1;
            let seed = random_seed();

            println!("Generating flower, seed: {seed}");
            let result = gen_flower_from_seed(seed, radius, blank_image_space_percent);

            match result {
                Ok(image) => image.save(format!("dev/flower/{seed}.png")).unwrap(),
                Err(e) => println!("failed to generate flower: {:?}", e),
            }
        }
    }
}

use crate::flower::random_flower;
use crate::graphics::{draw_flower, draw_mosaic};
use crate::mosaic::random_mosaic;
use crate::rand::RecoverableRng;
use ::rand::prelude::StdRng;
use ::rand::Rng;
use image::RgbImage;
use rand_core::SeedableRng;

mod color;
mod flower;
mod graphics;
mod math;
mod mosaic;
mod petal;
mod rand;

pub fn gen_mosaic(size: u16) -> Option<(RgbImage, u64)> {
    let seed = random_seed();
    gen_mosaic_from_seed(seed, size).map(|image| (image, seed))
}

pub fn gen_mosaic_from_seed(seed: u64, size: u16) -> Option<RgbImage> {
    let mut random = RecoverableRng::new(seed);
    let mosaic = random_mosaic(size / 2, &mut random)?;
    let image = draw_mosaic(&mosaic);
    Some(image)
}

pub fn gen_flower(size: u16) -> Option<(RgbImage, u64)> {
    let seed = random_seed();
    gen_flower_from_seed(seed, size).map(|image| (image, seed))
}

pub fn gen_flower_from_seed(seed: u64, size: u16) -> Option<RgbImage> {
    let mut random = RecoverableRng::new(seed);
    let flower = random_flower(size / 2, &mut random)?;
    let image = draw_flower(&flower);
    Some(image)
}

fn random_seed() -> u64 {
    StdRng::from_entropy().gen_range(u64::MIN..=u64::MAX)
}

#[cfg(test)]
mod tests {
    use crate::{gen_flower, gen_mosaic};
    use std::fs;

    #[test]
    fn mosaic() {
        fs::create_dir_all("dev/mosaic").unwrap();

        for _ in 0..10 {
            let size = 2500;

            if let Some((image, seed)) = gen_mosaic(size) {
                image.save(format!("dev/mosaic/{seed}.png")).unwrap();
            }
        }
    }

    #[test]
    fn flower() {
        fs::create_dir_all("dev/flower").unwrap();

        for _ in 0..10 {
            let size = 1200;

            if let Some((image, seed)) = gen_flower(size) {
                image.save(format!("dev/flower/{seed}.png")).unwrap();
            }
        }
    }
}

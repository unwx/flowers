use crate::color::{
    background_color, color_to_hsl, color_to_rgb, colorize, random_color, random_gradient,
    random_noise, random_palette,
};
use crate::math::normalize_f32;
use crate::petal::{find_petal_area, scale_and_merge_sides, side_sin, side_tan, MergeMode};
use glam::Vec2;
use image::RgbImage;
use palette::convert::FromColorUnclamped;
use palette::{Hsl, Srgb};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::f32::consts::PI;

mod color;
mod math;
mod petal;

pub fn gen_mosaic(size: u16) -> Option<(RgbImage, u64)> {
    let seed = StdRng::from_entropy().gen_range(u64::MIN..=u64::MAX);
    gen_mosaic_from_seed(seed, size).map(|image| (image, seed))
}

pub fn gen_mosaic_from_seed(seed: u64, size: u16) -> Option<RgbImage> {
    assert!(
        size >= 16 && i16::try_from(size).is_ok(),
        "illegal size '{size}', allowed: [16 <= size <= 32_767]"
    );
    assert_eq!(size % 2, 0, "size must be even");

    let mut random = StdRng::seed_from_u64(seed);
    let mut image = {
        let desired_size = (size as u32) + (size as u32 / 6);
        let size = desired_size.min(u16::MAX as u32);
        RgbImage::new(size, size)
    };

    {
        let mosaic_frame = {
            fn random_side<R: Rng>(rotation: f32, random: &mut R) -> Vec<Vec2> {
                let mirror = random.gen_bool(0.5);

                fn gen_step<R: Rng>(min_k: f32, max_k: f32, k: f32, random: &mut R) -> f32 {
                    let min_step = 0.001;
                    let max_step = 15.0;

                    random.gen_range(
                        min_step..=normalize_f32(k, min_k, max_k, max_step, max_step / 2.0),
                    )
                }

                if random.gen_bool(0.5) {
                    let min_k = 0.0001;
                    let max_k = 0.01;
                    let k = random.gen_range(min_k..=max_k);

                    let step = gen_step(min_k, max_k, k, random);
                    side_sin(k, step, rotation, mirror)
                } else {
                    let min_k = 0.00005;
                    let max_k = 0.005;
                    let k = random.gen_range(min_k..=max_k);

                    let step = gen_step(min_k, max_k, k, random);
                    side_tan(k, step, rotation, mirror)
                }
            }

            let rotation1 = random.gen_range(-PI..=PI);
            let rotation2 = random.gen_range(-PI..=PI);

            let side1 = random_side(rotation1, &mut random);
            let side2 = random_side(rotation2, &mut random);

            let merge_mode = if (rotation1 - rotation2).abs() <= 30.0_f32.to_radians() {
                MergeMode::SideWithSide
            } else {
                MergeMode::SideWithOrigin
            };

            scale_and_merge_sides(side1.as_slice(), side2.as_slice(), size / 2, merge_mode)
        };

        if mosaic_frame.is_empty() {
            return None;
        }

        let (colorful_points, primary_color) = {
            let mosaic_area = find_petal_area(mosaic_frame.as_slice());

            let gradient = {
                let start_color = random_color(&mut random);
                let palette = random_palette(random.gen_range(2..=6), start_color, &mut random);
                random_gradient(palette.as_slice(), &mut random)
            };
            let noise = random_noise(&mut random);

            colorize(
                mosaic_area.as_slice(),
                &gradient,
                &noise,
                random.gen_range(0.1..=7.5),
            )
        }?;

        let background_color = background_color(primary_color.clone(), &mut random);
        {
            let srgb = color_to_rgb(background_color.clone()).into_format::<u8>();
            for pixel in image.pixels_mut() {
                pixel.0[0] = srgb.red;
                pixel.0[1] = srgb.green;
                pixel.0[2] = srgb.blue;
            }
        }

        {
            let center = image.width() as u16 / 2;

            let light_acc =
                if color_to_hsl(background_color).lightness < 0.5 && random.gen_bool(0.5) {
                    2.0
                } else {
                    0.5
                };

            for (point, color) in colorful_points {
                let rgb = color.to_rgba8();
                image.put_pixel(
                    ((point.x as i32) + (center as i32)) as u32,
                    ((point.y as i32) + (center as i32)) as u32,
                    image::Rgb([rgb[0], rgb[1], rgb[2]]),
                )
            }

            for point in mosaic_frame {
                let x = ((point.x as i32) + (center as i32)) as u32;
                let y = ((point.y as i32) + (center as i32)) as u32;

                let pixel = image.get_pixel_mut(x, y);
                let pixel_srgb = Srgb::new(pixel.0[0], pixel.0[1], pixel.0[2]).into_format::<f32>();
                let mut pixel_hsl = Hsl::from_color_unclamped(pixel_srgb);

                pixel_hsl.lightness *= light_acc;
                let pixel_srgb = Srgb::from_color_unclamped(pixel_hsl).into_format::<u8>();

                pixel.0[0] = pixel_srgb.red;
                pixel.0[1] = pixel_srgb.green;
                pixel.0[2] = pixel_srgb.blue;
            }
        }
    }

    Some(image)
}

#[cfg(test)]
mod tests {
    use crate::gen_mosaic;
    use std::fs;

    #[test]
    fn it_works() {
        fs::create_dir_all("dev/mosaic").unwrap();

        for _ in 0..15 {
            let size = 2500;

            if let Some((image, seed)) = gen_mosaic(size) {
                image.save(format!("dev/mosaic/{seed}.png")).unwrap();
            }
        }
    }
}

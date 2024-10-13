use crate::flower::Flower;
use crate::graphics::color::convert::{hsl_to_rgb, image_rgb_to_rgb, rgb_to_hsl, rgb_to_image_rgb};
use crate::graphics::color::theory::find_average_color_iter;
use crate::math::real::{debug_assert_finite, debug_eval_finite};
use crate::mosaic::Mosaic;
use anyhow::{Context, Result};
use glam::{I16Vec2, UVec2};
use image::RgbImage;
use imageproc::drawing::draw_filled_circle_mut;
use palette::rgb::Rgb;
use palette::Hsl;

#[derive(Clone)]
pub struct Raster {
    pub skeleton: Vec<I16Vec2>,
    pub pixels: Vec<(I16Vec2, Rgb)>,
}

impl Raster {
    pub fn new(skeleton: Vec<I16Vec2>, pixels: Vec<(I16Vec2, Rgb)>) -> Self {
        Self { skeleton, pixels }
    }
}

pub fn draw_mosaic(mosaic: &Mosaic, blank_space_percent: f32) -> Result<RgbImage> {
    let mut image = create_image(radius_to_size(mosaic.radius()), blank_space_percent);
    let average_color = find_average_color_iter(mosaic.pixels.iter().map(|(_, rgb)| rgb))
        .context("failed to find average mosaic color")?;

    let background_color = create_background_color(average_color);
    fill_with_color(background_color, &mut image);

    draw(mosaic, is_background_dark(background_color), &mut image);
    Ok(image)
}

pub fn draw_flower(flower: &Flower, blank_space_percent: f32) -> Result<RgbImage> {
    let mut image = create_image(radius_to_size(flower.radius()), blank_space_percent);
    let average_color = find_average_color_iter(
        flower
            .petals()
            .iter()
            .map(|raster| &raster.pixels)
            .flat_map(|pixels| pixels.iter().map(|(_, color)| color)),
    )
    .context("failed to find average flower color")?;

    let background_color = create_background_color(average_color);
    let is_background_dark = is_background_dark(background_color);
    fill_with_color(background_color, &mut image);

    for petal in flower.petals() {
        draw(petal, is_background_dark, &mut image);
    }

    {
        let center = image_center(&image);
        let radius = {
            let desired = debug_eval_finite!(flower.mosaic().radius() as f32 * 1.03);
            desired
                .min((image.width().min(image.height()) / 2) as f32)
                .clamp(0.0, i32::MAX as f32 / 2.0) as i32
        };

        if radius > 1 {
            draw_filled_circle_mut(
                &mut image,
                (center.x as i32, center.y as i32),
                radius,
                rgb_to_image_rgb(background_color),
            );
        }
        draw(flower.mosaic(), is_background_dark, &mut image);
    }

    Ok(image)
}

fn draw(raster: &Raster, is_background_dark: bool, image: &mut RgbImage) {
    draw_pixels(raster.pixels.as_slice(), image);
    draw_skeleton(raster.skeleton.as_slice(), is_background_dark, image)
}

fn draw_skeleton(skeleton: &[I16Vec2], is_dark_background: bool, image: &mut RgbImage) {
    let (min_light, light_multiplier) = {
        if is_dark_background {
            (Some(0.15), 1.5)
        } else {
            (None, 0.75)
        }
    };

    for point in skeleton {
        if let Some(point) = centralize(*point, image) {
            if let Some(pixel) = image.get_pixel_mut_checked(point.x, point.y) {
                let mut pixel_hsl = rgb_to_hsl(image_rgb_to_rgb(*pixel));

                if let Some(min_light) = min_light {
                    pixel_hsl.lightness = pixel_hsl.lightness.max(min_light);
                }
                pixel_hsl.lightness *= light_multiplier;
                pixel_hsl.lightness = pixel_hsl.lightness.clamp(0.0, 1.0);

                *pixel = rgb_to_image_rgb(hsl_to_rgb(pixel_hsl))
            }
        }
    }
}

fn draw_pixels(pixels: &[(I16Vec2, Rgb)], image: &mut RgbImage) {
    for (point, rgb) in pixels {
        let rgb = rgb_to_image_rgb(*rgb);
        if let Some(point) = centralize(*point, image) {
            image.put_pixel(point.x, point.y, rgb)
        }
    }
}

fn fill_with_color(rgb: Rgb, image: &mut RgbImage) {
    let rgb = rgb_to_image_rgb(rgb);
    for pixel in image.pixels_mut() {
        *pixel = rgb;
    }
}

#[must_use]
fn is_background_dark(background: Rgb) -> bool {
    rgb_to_hsl(background).lightness <= 0.15
}

#[must_use]
fn create_background_color(average_color: Rgb) -> Rgb {
    let average_color = rgb_to_hsl(average_color);

    let hsl = if average_color.lightness > 0.15 {
        Hsl::new_srgb(average_color.hue, average_color.saturation, 0.95)
    } else {
        Hsl::new_srgb(average_color.hue, average_color.saturation, 0.05)
    };

    hsl_to_rgb(hsl)
}

#[must_use]
fn create_image(size: u16, blank_space_percent: f32) -> RgbImage {
    debug_assert_finite!(blank_space_percent);
    let blank_space_percent = blank_space_percent.clamp(0.0, 1.0);

    let desired_size = (size as u32) + ((size as f32 * blank_space_percent) as u32);
    let size = desired_size.min(u16::MAX as u32);
    RgbImage::new(size, size)
}

#[must_use]
fn centralize(point: I16Vec2, image: &RgbImage) -> Option<UVec2> {
    let center = image_center(image);
    let centralized_point = {
        if point.x.unsigned_abs() as u32 > center.x || point.y.unsigned_abs() as u32 > center.y {
            return None;
        }

        UVec2::new(
            u32::try_from(center.x as i64 + point.x as i64).ok()?,
            (image.height() - 1) - u32::try_from(center.y as i64 + point.y as i64).ok()?,
        )
    };

    Some(centralized_point)
}

#[must_use]
fn image_center(image: &RgbImage) -> UVec2 {
    (UVec2::new(image.width(), image.height()) - UVec2::ONE) / 2
}

fn radius_to_size(radius: u16) -> u16 {
    (radius as u32 * 2).min(u16::MAX as u32) as u16
}

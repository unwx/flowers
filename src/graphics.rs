use crate::color::{color_to_hsl, color_to_image_rgb, hsl_to_color, image_rgb_to_color};
use crate::flower::Flower;
use crate::mosaic::Mosaic;
use colorgrad::Color;
use glam::{I16Vec2, Mat2, UVec2};
use image::RgbImage;
use imageproc::drawing::draw_filled_circle_mut;
use std::ops::{AddAssign, MulAssign};

pub struct Drawing {
    pub skeleton: Vec<I16Vec2>,
    pub pixels: Vec<(I16Vec2, Color)>,
    pub average_color: Color,
}

impl Drawing {
    fn shift_points<F>(&mut self, mut func: F)
    where
        F: FnMut(&mut I16Vec2),
    {
        self.skeleton.iter_mut().for_each(&mut func);
        self.pixels.iter_mut().for_each(|(point, _)| func(point));
    }
}

impl AddAssign<I16Vec2> for Drawing {
    fn add_assign(&mut self, rhs: I16Vec2) {
        if rhs == I16Vec2::ZERO {
            return;
        }

        self.shift_points(|point| *point += rhs);
    }
}

impl MulAssign<Mat2> for Drawing {
    fn mul_assign(&mut self, rhs: Mat2) {
        self.shift_points(|point| *point = rhs.mul_vec2(point.as_vec2()).round().as_i16vec2());
    }
}

pub fn draw_mosaic(mosaic: &Mosaic) -> RgbImage {
    let mut image = create_image_with_extra_size(mosaic.size);
    fill_with_color(mosaic.background_color.clone(), &mut image);
    draw(&mosaic, mosaic.background_color.clone(), &mut image);
    image
}

pub fn draw_flower(flower: &Flower) -> RgbImage {
    let mut image = create_image_with_extra_size(flower.size);
    fill_with_color(flower.background_color.clone(), &mut image);

    for layer in &flower.layers {
        for petal in &layer.petals {
            draw(petal, flower.background_color.clone(), &mut image);
        }
    }

    {
        let center = get_image_center(&image);
        let image_size = image.width() as i32;

        draw_filled_circle_mut(
            &mut image,
            (center as i32, center as i32),
            ((flower.mosaic.size as f32 * 0.075) as i32).min(image_size),
            color_to_image_rgb(flower.background_color.clone()),
        );
        draw(&flower.mosaic, flower.background_color.clone(), &mut image);
    }

    image
}

fn draw(drawing: &Drawing, background_color: Color, image: &mut RgbImage) {
    draw_pixels(drawing.pixels.as_slice(), image);
    draw_skeleton(drawing.skeleton.as_slice(), background_color, image)
}

fn draw_skeleton(skeleton: &[I16Vec2], background_color: Color, image: &mut RgbImage) {
    let center = get_image_center(image);
    let (initial_min_light, light_multiplier) = {
        if color_to_hsl(background_color).lightness < 0.25 {
            (Some(0.15), 1.5)
        } else {
            (None, 0.75)
        }
    };

    for point in skeleton {
        let point = centralize(*point, center);
        let pixel = image.get_pixel_mut(point.x, point.y);
        let mut pixel_hsl = color_to_hsl(image_rgb_to_color(pixel.clone()));

        if let Some(light) = initial_min_light {
            pixel_hsl.lightness = pixel_hsl.lightness.max(light);
        }
        pixel_hsl.lightness = (pixel_hsl.lightness * light_multiplier).clamp(0.0, 1.0);

        *pixel = color_to_image_rgb(hsl_to_color(pixel_hsl))
    }
}

fn draw_pixels(pixels: &[(I16Vec2, Color)], image: &mut RgbImage) {
    let center = get_image_center(image);
    for (point, color) in pixels {
        let rgb = color_to_image_rgb(color.clone());
        let point = centralize(*point, center);
        image.put_pixel(point.x, point.y, rgb)
    }
}

fn fill_with_color(color: Color, image: &mut RgbImage) {
    let rgb = color_to_image_rgb(color);
    for pixel in image.pixels_mut() {
        *pixel = rgb.clone();
    }
}

fn create_image_with_extra_size(active_size: u16) -> RgbImage {
    let desired_size = (active_size as u32) + ((active_size as f32 * 0.1) as u32);
    let size = desired_size.min(u16::MAX as u32);
    RgbImage::new(size, size)
}

fn centralize(point: I16Vec2, center: u16) -> UVec2 {
    (point + center as i16).as_uvec2().clamp(UVec2::ZERO, UVec2::splat((center as u32 * 2) - 1))
}

fn get_image_center(image: &RgbImage) -> u16 {
    image.width() as u16 / 2
}

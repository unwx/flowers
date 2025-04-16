use crate::art::TexturedArea;
use crate::color::convert::color_to_image_rgb;
use crate::math::definition::IntPoint;
use crate::util::macros::debug_assert_finite;
use colorgrad::Gradient;
use glam::{I64Vec2, UVec2};
use image::{GenericImageView, Rgba, RgbaImage};
use noise::NoiseFn;

pub fn draw_texture<G, N>(texture: &TexturedArea<G, N>, image: &mut RgbaImage)
where
    G: Gradient,
    N: NoiseFn<f64, 2>,
{
    let area = texture.area();
    let total_elements = area.coverage();
    debug_assert!(total_elements > 0);

    let min_y = area.min_y();
    let max_y = area.max_y();
    let min_x = area.min_x();
    let max_x = area.max_x();

    // Copy & paste from noise::utils::noise_map_builder::PlaneMapBuilder
    // Generating noise is expensive on the CPU,
    // so we generate noise only within our `area` for performance reasons.
    let noise_map = {
        let mut noise_values = Vec::with_capacity(total_elements);
        let noise_scale = texture.noise_scale().abs() as f64;
        let extent = noise_scale * 2.0;
        debug_assert_finite!(extent);

        let width = (max_x - min_x) as usize + 1;
        let height = (max_y - min_y) as usize + 1;

        let x_step = extent / width as f64;
        let y_step = extent / height as f64;
        debug_assert_finite!(x_step, y_step);

        for (y, line) in area.enumerate() {
            let noise_y = -noise_scale + (y_step * (y - min_y) as f64);
            debug_assert_finite!(noise_y);

            for x in line.flatten() {
                if let Some(pixel_pos) = point_to_pixel_pos(IntPoint::new(x, y), image) {
                    let noise_x = -noise_scale + (x_step * (x - min_x) as f64);
                    debug_assert_finite!(noise_x);

                    let mut noise_value = texture.noise().get([noise_x, noise_y]);
                    debug_assert_finite!(noise_value);

                    if !noise_value.is_finite() {
                        noise_value = 0.0;
                    }
                    noise_values.push((pixel_pos, noise_value))
                }
            }
        }

        debug_assert_eq!(noise_values.len(), total_elements);
        noise_values
    };

    let (min_noise_value, max_noise_value) = {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for &(_, value) in &noise_map {
            min = min.min(value);
            max = max.max(value);
        }

        if !min.is_finite() || !max.is_finite() {
            panic!("total_elements > 0, there must be at least one finite noise value");
        }

        (min, max)
    };

    if max_noise_value - min_noise_value > f64::EPSILON {
        for (pos, noise_value) in noise_map {
            let rgb = color_to_image_rgb(
                texture
                    .palette()
                    .at_color(noise_value, min_noise_value..=max_noise_value),
            );
            image.put_pixel(pos.x, pos.y, rgb);
        }
    } else {
        let rgb = color_to_image_rgb(texture.palette().at_color(0.0, 0.0..=1.0));
        for (pos, _) in noise_map {
            image.put_pixel(pos.x, pos.y, rgb);
        }
    }
}


pub fn clear_points<I>(points: I, image: &mut RgbaImage)
where
    I: IntoIterator<Item = IntPoint>,
{
    for point in points {
        clear_point(point, image);
    }
}

pub fn clear_point(point: IntPoint, image: &mut RgbaImage) {
    if let Some(pos) = point_to_pixel_pos(point, image) {
        image.put_pixel(pos.x, pos.y, null_rgb())
    }
}


pub fn point_to_pixel_pos(point: IntPoint, image: &RgbaImage) -> Option<UVec2> {
    Some(image_center(image))
        .map(|center| center.as_i64vec2() + point.as_i64vec2())
        .map(|it| I64Vec2::new(it.x, (image.height() as i64 - 1) - it.y))
        .and_then(|it| UVec2::try_from(it).ok())
        .filter(|it| image.in_bounds(it.x, it.y))
}

fn image_center(image: &RgbaImage) -> UVec2 {
    (UVec2::new(image.width(), image.height()) - UVec2::ONE) / 2
}

fn null_rgb() -> Rgba<u8> {
    Rgba::from([0, 0, 0, 0])
}

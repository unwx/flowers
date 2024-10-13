use crate::graphics::color::convert::color_to_rgb;
use crate::math::area::Area;
use crate::math::normalize;
use crate::math::real::debug_assert_finite;
use colorgrad::Gradient;
use glam::I16Vec2;
use noise::NoiseFn;
use palette::rgb::Rgb;

pub fn color_area<G, N>(
    area: &Area,
    gradient: &G,
    noise: &N,
    noise_scale: f32,
) -> Vec<(I16Vec2, Rgb)>
where
    G: Gradient,
    N: NoiseFn<f64, 2>,
{
    debug_assert_finite!(noise_scale);

    let total_elements = area.coverage();
    if total_elements == 0 {
        return vec![];
    }

    let min_y = area.min_y();
    let max_y = area.max_y();
    let (min_x, max_x) = {
        let min = area.min_x();
        let max = area.max_x();

        if min.is_none() || max.is_none() {
            panic!("total_elements > 0, there must be at least one 'X'");
        }

        (min.unwrap(), max.unwrap())
    };

    let noise_map = {
        let mut noise_values = Vec::with_capacity(total_elements);
        let noise_scale = noise_scale.abs() as f64;
        let extent = noise_scale * 2.0;
        debug_assert_finite!(extent);

        let width = (max_x - min_x) as usize + 1;
        let height = (max_y - min_y) as usize + 1;

        let x_step = extent / width as f64;
        let y_step = extent / height as f64;
        debug_assert_finite!(x_step, y_step);

        for (y, line) in area.iter() {
            let noise_y = -noise_scale + (y_step * (y - min_y) as f64);
            debug_assert_finite!(noise_y);

            for x in line.iter_x() {
                let noise_x = -noise_scale + (x_step * (x - min_x) as f64);
                debug_assert_finite!(noise_x);

                let mut noise_value = noise.get([noise_x, noise_y]);
                debug_assert_finite!(noise_value);

                if !noise_value.is_finite() {
                    noise_value = 0.0;
                }
                noise_values.push((I16Vec2::new(x, y), noise_value))
            }
        }

        noise_values
    };

    debug_assert_eq!(noise_map.len(), total_elements);
    let (min_noise_value, max_noise_value) = {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;

        for (_, value) in &noise_map {
            min = f64::min(min, *value);
            max = f64::max(max, *value);
        }

        if !min.is_finite() || !max.is_finite() {
            panic!("total_elements > 0, there must be at least one noise value");
        }

        (min, max)
    };

    let mut pixels = Vec::with_capacity(total_elements);
    if min_noise_value == max_noise_value || gradient.domain().0 == gradient.domain().1 {
        let rgb = color_to_rgb(gradient.at(gradient.domain().0));
        for (point, _) in noise_map {
            pixels.push((point, rgb));
        }
    } else {
        for (point, noise_value) in noise_map {
            let gradient_value = normalize(
                noise_value,
                min_noise_value,
                max_noise_value,
                gradient.domain().0 as f64,
                gradient.domain().1 as f64,
            )
            .clamp(f32::MIN as f64, f32::MAX as f64) as f32;

            let rgb = color_to_rgb(gradient.at(gradient_value));
            pixels.push((point, rgb));
        }
    }

    debug_assert_eq!(pixels.len(), total_elements);
    pixels
}

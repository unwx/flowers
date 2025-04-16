use crate::util::macros::debug_assert_finite;
use colorgrad::Color;
use palette::{Clamp, FromColor, Okhsl, Srgb};

pub fn color_to_image_rgb(color: Color) -> image::Rgba<u8> {
    let rgb = Srgb::new(color.r, color.g, color.b).into_format::<u8>();
    image::Rgba::<u8>::from([rgb.red, rgb.green, rgb.blue, u8::MAX])
}

pub fn hsl_to_color(hsl: Okhsl) -> Color {
    debug_assert_finite!(hsl);
    debug_assert_eq!(hsl, hsl.clamp());

    let rgb = Srgb::from_color(hsl);
    Color::new(rgb.red, rgb.green, rgb.blue, 1.0)
}

use crate::math::real::debug_assert_finite;
use colorgrad::Color;
use palette::convert::FromColorUnclamped;
use palette::rgb::Rgb;
use palette::{Clamp, Hsl};

#[must_use]
pub fn rgb_to_color(rgb: Rgb) -> Color {
    debug_assert_finite!(rgb);
    debug_assert_eq!(rgb, rgb.clamp());
    Color::new(rgb.red, rgb.green, rgb.blue, 1.0)
}

#[must_use]
pub fn rgb_to_image_rgb(rgb: Rgb) -> image::Rgb<u8> {
    debug_assert_finite!(rgb);
    debug_assert_eq!(rgb, rgb.clamp());
    let rgb = rgb.into_format::<u8>();
    image::Rgb([rgb.red, rgb.green, rgb.blue])
}

#[must_use]
pub fn rgb_to_hsl(rgb: Rgb) -> Hsl {
    debug_assert_finite!(rgb);
    debug_assert_eq!(rgb, rgb.clamp());
    Hsl::from_color_unclamped(rgb)
}

#[must_use]
pub fn hsl_to_rgb(hsl: Hsl) -> Rgb {
    debug_assert_finite!(hsl);
    debug_assert_eq!(hsl, hsl.clamp());
    Rgb::from_color_unclamped(hsl)
}

#[must_use]
pub fn color_to_rgb(color: Color) -> Rgb {
    debug_assert_finite!(color.clone());
    fn assert_clamped(value: f32) {
        debug_assert_eq!(value, value.clamp(0.0, 1.0));
    }
    
    assert_clamped(color.r);
    assert_clamped(color.g);
    assert_clamped(color.b);
    Rgb::new(color.r, color.g, color.b)
}

#[must_use]
pub fn image_rgb_to_rgb(rgb: image::Rgb<u8>) -> Rgb {
    let converted = Rgb::new(rgb.0[0], rgb.0[1], rgb.0[2]);
    converted.into_format::<f32>()
}

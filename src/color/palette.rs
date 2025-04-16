use crate::color::convert::color_to_image_rgb;
use crate::math::remap;
use crate::util::macros::debug_assert_finite;
use colorgrad::{Color, Gradient};
use image::buffer::ConvertBuffer;
use image::{ImageBuffer, RgbImage};
use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub struct Palette<G: Gradient> {
    gradient: G,
}

impl<G: Gradient> Palette<G> {
    pub fn new(gradient: G) -> Self {
        Self { gradient }
    }

    pub fn at_color(&self, position: f64, position_range: RangeInclusive<f64>) -> Color {
        debug_assert_finite!(position);
        debug_assert!(position_range.contains(&position));

        let domain = self.gradient.domain();
        let index = remap(
            position.clamp(*position_range.start(), *position_range.end()),
            *position_range.start(),
            *position_range.end(),
            domain.0 as f64,
            domain.1 as f64,
        ) as f32;

        self.gradient.at(index)
    }

    #[allow(dead_code)]
    pub fn to_image(&self, width: u32, height: u32) -> RgbImage {
        ImageBuffer::from_fn(width, height, |x, _| {
            color_to_image_rgb(self.at_color(x as f64, 0.0..=((width - 1) as f64)))
        })
        .convert()
    }
}

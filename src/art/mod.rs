use crate::color::palette::Palette;
use crate::math::area::Area;
use colorgrad::Gradient;
use noise::NoiseFn;

pub mod color;
pub mod dynamic;
pub mod flower;
pub mod gradient;
pub mod mosaic;
pub mod pattern;

#[derive(Debug, Clone)]
pub struct TexturedArea<G, N>
where
    G: Gradient,
    N: NoiseFn<f64, 2>,
{
    area: Area,
    palette: Palette<G>,
    noise: N,
    noise_scale: f32,
}

impl<G, N> TexturedArea<G, N>
where
    G: Gradient,
    N: NoiseFn<f64, 2>,
{
    pub(crate) fn new(area: Area, palette: Palette<G>, noise: N, noise_scale: f32) -> Self {
        assert!(noise_scale > 0.0, "noise_scale must be > 0.0");

        Self {
            area,
            palette,
            noise,
            noise_scale,
        }
    }

    pub fn area(&self) -> &Area {
        &self.area
    }

    pub fn palette(&self) -> &Palette<G> {
        &self.palette
    }

    pub fn noise(&self) -> &N {
        &self.noise
    }

    pub fn noise_scale(&self) -> f32 {
        self.noise_scale
    }
}

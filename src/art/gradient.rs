use crate::art::dynamic::DynGradient;
use anyhow::{bail, Context, Result};
use colorgrad::{BlendMode, Color, GradientBuilder, LinearGradient};

pub trait GradientConverter {
    fn colors_to_gradient(&self, colors: Vec<Color>, seed: u64) -> Result<DynGradient>;
}


#[derive(Debug, Copy, Clone)]
pub struct LinearGradientConverter;

impl GradientConverter for LinearGradientConverter {
    fn colors_to_gradient(&self, colors: Vec<Color>, _: u64) -> Result<DynGradient> {
        if colors.len() < 2 {
            bail!("there must be at least 2 colors to build a gradient");
        }

        let gradient = GradientBuilder::new()
            .mode(BlendMode::Rgb)
            .colors(colors.as_slice())
            .build::<LinearGradient>()
            .context("failed to build LinearGradient")?;

        Ok(DynGradient::new(gradient))
    }
}

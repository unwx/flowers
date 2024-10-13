use crate::graphics::color::convert::rgb_to_color;
use colorgrad::{
    BasisGradient, BlendMode, CatmullRomGradient, Color, Gradient, GradientBuilder, LinearGradient,
    SharpGradient,
};
use palette::rgb::Rgb;
use rand::Rng;
use std::ops::Deref;

pub enum DynGradient {
    Linear(LinearGradient),
    Basis(BasisGradient),
    CatmullRom(CatmullRomGradient),
    Sharp(SharpGradient),
}

impl Deref for DynGradient {
    type Target = dyn Gradient;

    fn deref(&self) -> &Self::Target {
        match self {
            DynGradient::Linear(g) => g,
            DynGradient::Basis(g) => g,
            DynGradient::CatmullRom(g) => g,
            DynGradient::Sharp(g) => g,
        }
    }
}

impl Gradient for DynGradient {
    fn at(&self, t: f32) -> Color {
        self.deref().at(t)
    }

    fn repeat_at(&self, t: f32) -> Color {
        self.deref().repeat_at(t)
    }

    fn reflect_at(&self, t: f32) -> Color {
        self.deref().reflect_at(t)
    }

    fn domain(&self) -> (f32, f32) {
        self.deref().domain()
    }

    fn colors(&self, n: usize) -> Vec<Color> {
        self.deref().colors(n)
    }

    fn sharp(&self, segment: u16, smoothness: f32) -> SharpGradient {
        self.deref().sharp(segment, smoothness)
    }
}

#[must_use]
pub fn random_gradient<R: Rng>(colors: &[Rgb], random: &mut R) -> DynGradient {
    assert!(
        colors.len() > 1,
        "to build a gradient, you must provide at least two colors"
    );

    let mut builder = GradientBuilder::new();
    let stages = builder.mode(BlendMode::Rgb).colors(
        colors
            .iter()
            .map(|rgb| rgb_to_color(*rgb))
            .collect::<Vec<Color>>()
            .as_slice(),
    );
    let gradient_choice = random.gen_range(0..4);

    match gradient_choice {
        0 => stages.build::<LinearGradient>().map(DynGradient::Linear),
        1 => stages.build::<BasisGradient>().map(DynGradient::Basis),
        2 => stages
            .build::<CatmullRomGradient>()
            .map(DynGradient::CatmullRom),
        3 => stages
            .build::<LinearGradient>()
            .map(|g| g.sharp(random.gen_range(2..=32), random.gen_range(0.0..=0.75)))
            .map(DynGradient::Sharp),
        _ => unreachable!(),
    }
    .unwrap_or_else(|e| {
        panic!(
            "bug: failed to build a gradient: {}. \
            [gradient_choice: {}, colors: {:?}]",
            e, gradient_choice, colors
        )
    })
}

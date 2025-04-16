use colorgrad::{Color, Gradient, InverseGradient, SharpGradient};
use noise::NoiseFn;

pub struct DynNoise<T, const DIM: usize>(Box<dyn NoiseFn<T, DIM>>);

impl<T, const DIM: usize> DynNoise<T, DIM> {
    pub fn new<N: NoiseFn<T, DIM> + 'static>(noise: N) -> Self {
        Self(Box::new(noise))
    }
}

impl<T, const DIM: usize> NoiseFn<T, DIM> for DynNoise<T, DIM> {
    fn get(&self, point: [T; DIM]) -> f64 {
        self.0.get(point)
    }
}


pub struct DynGradient(Box<dyn Gradient>);

impl DynGradient {
    pub fn new<G: Gradient + 'static>(gradient: G) -> Self {
        Self(Box::new(gradient))
    }
}

impl Clone for DynGradient {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Gradient for DynGradient {
    fn at(&self, t: f32) -> Color {
        self.0.at(t)
    }

    fn repeat_at(&self, t: f32) -> Color {
        self.0.repeat_at(t)
    }

    fn reflect_at(&self, t: f32) -> Color {
        self.0.reflect_at(t)
    }

    fn domain(&self) -> (f32, f32) {
        self.0.domain()
    }

    fn colors(&self, n: usize) -> Vec<Color> {
        self.0.colors(n)
    }

    fn sharp(&self, segment: u16, smoothness: f32) -> SharpGradient {
        self.0.sharp(segment, smoothness)
    }

    fn boxed<'a>(self) -> Box<dyn Gradient + 'a>
    where
        Self: Sized + 'a,
    {
        self.0.clone()
    }

    fn inverse<'a>(&self) -> InverseGradient
    where
        Self: 'a,
    {
        self.0.inverse()
    }
}

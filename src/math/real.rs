use glam::Vec2;
use palette::{Lab, Okhsl};

pub trait IsFinite
where
    Self: Copy,
{
    fn is_finite(self) -> bool;
}

impl<T: IsFinite> IsFinite for &T {
    fn is_finite(self) -> bool {
        T::is_finite(*self)
    }
}

impl IsFinite for f32 {
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }
}

impl IsFinite for f64 {
    fn is_finite(self) -> bool {
        f64::is_finite(self)
    }
}

impl IsFinite for Vec2 {
    fn is_finite(self) -> bool {
        Vec2::is_finite(self)
    }
}

impl IsFinite for Okhsl {
    fn is_finite(self) -> bool {
        f32::from(self.hue).is_finite() && self.saturation.is_finite() && self.lightness.is_finite()
    }
}

impl IsFinite for Lab {
    fn is_finite(self) -> bool {
        self.l.is_finite() && self.a.is_finite() && self.b.is_finite()
    }
}

impl IsFinite for image::Rgba<f32> {
    fn is_finite(self) -> bool {
        self.0.into_iter().all(|v| v.is_finite())
    }
}

use glam::Vec2;

pub fn to_cartesian(
    radius: f32,
    theta: f32,
) -> Vec2 {
    Vec2::new(radius * theta.cos(), radius * theta.sin())
}

pub fn normalize_f64(value: f64, old_min: f64, old_max: f64, new_min: f64, new_max: f64) -> f64 {
    new_min + (value - old_min) * (new_max - new_min) / (old_max - old_min)
}


pub trait Rotate: Copy + Clone {
    fn rotate_radians(self, angle: f32) -> Self;
}

impl Rotate for Vec2 {
    fn rotate_radians(self, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Vec2::new(
            self.x * cos - self.y * sin,
            self.x * sin + self.y * cos,
        )
    }
}

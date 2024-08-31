use glam::Vec2;

pub fn to_cartesian(
    radius: f32,
    theta: f32,
) -> Vec2 {
    Vec2::new(radius * theta.cos(), radius * theta.sin())
}


pub trait Rotate {
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

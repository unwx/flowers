use glam::{I16Vec2, Vec2};

pub type Int = i16; // By current design, Int **cannot** be greater than i32
pub type Scalar = f32; // By current design, Scalar **should not** be greater than f64
pub type Point = Vec2;
pub type IntPoint = I16Vec2;


pub trait PointExtensions
where
    Self: Copy,
{
    fn as_int_point(self) -> IntPoint;
}

impl PointExtensions for Point {
    fn as_int_point(self) -> IntPoint {
        self.as_i16vec2()
    }
}


pub trait IntPointExtensions
where
    Self: Copy,
{
    fn as_point(self) -> Point;

    fn is_near_to(self, other: Self) -> bool;
}

impl IntPointExtensions for IntPoint {
    fn as_point(self) -> Point {
        self.as_vec2()
    }

    fn is_near_to(self, other: Self) -> bool {
        let diff = (self - other).abs();
        diff.x <= 1 && diff.y <= 1
    }
}

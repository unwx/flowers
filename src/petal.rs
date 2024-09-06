use crate::math::{to_cartesian, Rotate};
use glam::Vec2;
use std::f32::consts::PI;

pub fn petal_side_sin(k: f32, step: f32, flip: bool, mirror: bool) -> Vec<Vec2> {
    petal_side(k, step, flip, mirror, f32::sin, f32::asin)
}

pub fn petal_side_tan(k: f32, step: f32, flip: bool, mirror: bool) -> Vec<Vec2> {
    petal_side(k, step, flip, mirror, f32::tan, f32::atan)
}


fn petal_side<F, AF>(
    k: f32,
    step: f32,
    flip: bool,
    mirror: bool,
    trig_func: F,
    arc_trig_func: AF
) -> Vec<Vec2> where
    F: Fn(f32) -> f32,
    AF: Fn(f32) -> f32
{
    let theta_bound = arc_trig_func(1.0) / k;
    let mut petal = Vec::with_capacity((theta_bound / step) as usize + 1);

    if petal.capacity() == 0 {
        return petal;
    }

    for i in 0..petal.capacity() {
        let theta = (i as f32) * step;
        petal.push(to_cartesian(trig_func(theta * k), theta));
    }

    {
        let max_point = petal.last().unwrap();
        let rotation = if flip { max_point.angle_to(Vec2::Y) } else { PI + max_point.angle_to(Vec2::Y) };

        for i in 0..petal.len() {
            petal[i] = petal[i].rotate_radians(rotation);
        }
    }

    if flip {
        for i in 0..petal.len() {
            petal[i] -= Vec2::Y
        }
    }
    if mirror {
        for i in 0..petal.len() {
            petal[i].x = -petal[i].x
        }
    }

    petal
}

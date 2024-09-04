use crate::math::{to_cartesian, Rotate};
use glam::Vec2;
use std::f32::consts::PI;

pub fn petal_sin(k: f32, step: f32, flip: bool) -> Vec<Vec2> {
    let theta_bound = 2.0 * (PI / (2.0 * k));
    let mut petal = Vec::with_capacity((theta_bound / step) as usize + 1);

    eval_polar_func(k, step, 1.0, petal.capacity(), false, flip, f32::sin, &mut petal);
    petal
}

pub fn petal_tan(k1: f32, k2: f32, step: f32, flip: bool) -> Vec<Vec2> {
    let eval_bound = |k: f32| -> f32 { PI / (4.0 * k) };
    let eval_capacity = |bound: f32| -> usize { (bound / step) as usize + 1 };

    let capacity1 = eval_capacity(eval_bound(k1));
    let capacity2 = eval_capacity(eval_bound(k2));
    let total_capacity = capacity1 + capacity2;

    let mut petal = Vec::with_capacity(total_capacity);
    eval_polar_func(k1, step, 0.5, capacity1, false, flip, f32::tan, &mut petal);
    eval_polar_func(k2, step, 0.5, capacity2, true, flip, f32::tan, &mut petal);

    for i in capacity1..total_capacity {
        petal[i].x = -petal[i].x;
    }

    petal
}

fn eval_polar_func<F>(
    k: f32,
    step: f32,
    simple_func_k: f32,
    size: usize,
    reverse_values_order: bool,
    flip_values: bool,
    trig_func: F,
    destination: &mut Vec<Vec2>,
) where
    F: Fn(f32) -> f32,
{
    let mut max_point = Vec2::ZERO;
    let mut max_length = 0.0;

    let start_index = destination.len();

    for i in 0..size {
        let reversed_i = if reverse_values_order { size - i - 1 } else { i };
        let theta = reversed_i as f32 * step;
        let radius = trig_func(theta * k);

        let point = to_cartesian(radius, theta);
        if max_length < radius {
            max_length = radius;
            max_point = point;
        }

        destination.push(point);
    }

    let should_flip = k < simple_func_k || flip_values;
    let shift = if should_flip { max_point.angle_to(Vec2::Y) } else { PI + max_point.angle_to(Vec2::Y) };

    for i in start_index..destination.len() {
        destination[i] = destination[i].rotate_radians(shift);
    }

    if should_flip {
        for i in start_index..destination.len() {
            destination[i] = destination[i] - Vec2::Y;
        }
    }
}

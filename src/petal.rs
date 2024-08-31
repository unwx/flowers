use std::f32::consts::PI;
use glam::Vec2;
use crate::math::{to_cartesian, Rotate};

pub fn petal_sin(k: f32, step: f32) -> Vec<Vec2> {
    let theta_bound = 2.0 * (PI / (2.0 * k));
    (0..=(theta_bound / step) as usize)
        .map(|i| {
            let theta = i as f32 * step;
            let radius = (theta * k).sin();
            to_cartesian(radius, theta)
        })
        .collect()
}

pub fn petal_tan(k1: f32, k2: f32, step: f32) -> Vec<Vec2> {
    let eval_bound = |k: f32| -> f32 {
        PI / (4.0 * k)
    };
    let eval_capacity = |bound: f32| -> usize {
        (bound / step) as usize + 1
    };

    let theta_bound1 = eval_bound(k1);
    let theta_bound2 = eval_bound(k2);

    let capacity1 = eval_capacity(theta_bound1);
    let capacity2 = eval_capacity(theta_bound2);
    let total_capacity = capacity1 + capacity2;

    let mut points = Vec::with_capacity(total_capacity);
    let mut eval_polar_func = |k: f32, times: usize| {
        for i in 0..times {
            let theta = i as f32 * step;
            let radius = (theta * k).tan();
            points.push(to_cartesian(radius, theta));
        }
    };

    eval_polar_func(k1, capacity1);
    eval_polar_func(k2, capacity2);

    for i in capacity1..total_capacity {
        points[i].x = -points[i].x;
    }

    let angle_diff = {
        let angle1 = points[capacity1 - 1].angle_to(Vec2::X);
        let angle2 = points[total_capacity - 1].angle_to(Vec2::X);
        angle2 - angle1
    };

    for i in capacity1..total_capacity {
        points[i] = points[i].rotate_radians(angle_diff);
    }

    points
}

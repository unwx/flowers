use crate::math::real::{debug_assert_finite, debug_eval_finite, FiniteChecker};
use glam::{I16Vec2, Vec2};
use std::ops::{Add, Div, Mul, Sub};

pub(crate) mod area;
pub(crate) mod curve;
pub(crate) mod real;

#[must_use]
pub(crate) fn to_cartesian(radius: f32, theta: f32) -> Vec2 {
    debug_assert_finite!(radius, theta);
    Vec2::new(
        debug_eval_finite!(radius * theta.cos()),
        debug_eval_finite!(radius * theta.sin()),
    )
}

#[must_use]
pub(crate) fn normalize<T>(value: T, old_min: T, old_max: T, new_min: T, new_max: T) -> T
where
    T: Copy
        + Add<T, Output = T>
        + Sub<T, Output = T>
        + Mul<T, Output = T>
        + Div<T, Output = T>
        + FiniteChecker,
{
    debug_assert_finite!(value, old_min, old_max, new_min, new_max);
    debug_eval_finite!(new_min + ((value - old_min) * (new_max - new_min) / (old_max - old_min)))
}

#[must_use]
pub(crate) fn interpolate(points: &[I16Vec2]) -> Vec<I16Vec2> {
    if points.is_empty() {
        return vec![];
    }

    let mut result = Vec::with_capacity(points.len());
    result.push(points[0]);

    for i in 1..points.len() {
        let point = points[i];
        let previous_point = points[i - 1];

        let diff = (point - previous_point).as_vec2();
        let steps = diff.x.abs().max(diff.y.abs());
        let mut step = 1.0;

        while step <= steps {
            let progress = step / steps;
            let interpolated_point = previous_point + debug_eval_finite!((diff * progress).round()).as_i16vec2();

            result.push(interpolated_point);
            step += 1.0;
        }
    }

    result
}

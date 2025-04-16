use crate::math::definition::{IntPoint, IntPointExtensions, PointExtensions};
use crate::math::real::IsFinite;
use crate::util::macros::{debug_assert_finite, debug_eval_finite};
use num_traits::Float;
use std::fmt::Debug;

pub(crate) mod area;
pub(crate) mod curve;
pub(crate) mod definition;
pub(crate) mod grid;
pub(crate) mod polar;
pub(crate) mod real;

pub use area::line::Line;
pub use area::line::LineError;
pub use area::merge;
pub use area::range::Range;
pub use area::Area;

pub(crate) fn remap<T>(value: T, old_min: T, old_max: T, new_min: T, new_max: T) -> T
where
    T: Float + IsFinite + Debug,
{
    debug_assert_finite!(value, old_min, old_max, new_min, new_max);
    debug_assert_ne!(old_min, old_max);

    let remapped_value = new_min + ((value - old_min) * (new_max - new_min) / (old_max - old_min));
    debug_eval_finite!(remapped_value)
}

pub(crate) fn remap_bias<T>(value: T, old_min: T, old_max: T, new_min: T, new_max: T, bias: T) -> T
where
    T: Float + IsFinite + Debug,
{
    debug_assert_finite!(value, old_min, old_max, new_min, new_max, bias);
    debug_assert_ne!(old_min, old_max);

    let normalized_value = ((value - old_min) / (old_max - old_min)).clamp(T::zero(), T::one());
    let biased_normalized_value = if bias == T::zero() {
        normalized_value
    } else {
        let exponent = (-bias).exp();
        normalized_value.powf(exponent)
    };

    let remapped_value = new_min + biased_normalized_value * (new_max - new_min);
    debug_eval_finite!(remapped_value)
}

pub(crate) fn interpolate(points: &[IntPoint]) -> Vec<IntPoint> {
    if points.is_empty() {
        return vec![];
    }

    let mut result = Vec::with_capacity(points.len());
    result.push(points[0]);

    for i in 1..points.len() {
        let point = points[i];
        let past_point = points[i - 1];

        let diff = (point - past_point).as_point();
        let steps = diff.x.abs().max(diff.y.abs());
        let mut step = 1.0;

        while step <= steps {
            let progress = step / steps;
            let interpolated_point =
                past_point + debug_eval_finite!((diff * progress).round()).as_int_point();

            result.push(interpolated_point);
            step += 1.0;
        }
    }

    result
}

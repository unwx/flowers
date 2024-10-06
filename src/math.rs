use glam::Vec2;
use std::cmp::Ordering;
use std::f32::consts::PI;

pub fn to_cartesian(radius: f32, theta: f32) -> Vec2 {
    Vec2::new(radius * theta.cos(), radius * theta.sin())
}

pub fn normalize_f64(value: f64, old_min: f64, old_max: f64, new_min: f64, new_max: f64) -> f64 {
    new_min + (value - old_min) * (new_max - new_min) / (old_max - old_min)
}

pub fn normalize_f32(value: f32, old_min: f32, old_max: f32, new_min: f32, new_max: f32) -> f32 {
    normalize_f64(
        value as f64,
        old_min as f64,
        old_max as f64,
        new_min as f64,
        new_max as f64,
    ) as f32
}

pub fn wrap_radians(value: f32) -> f32 {
    value.rem_euclid(PI * 2.0)
}

pub fn find_nearest_f32<T, F>(values: &[T], target: f32, to_f32_func: F) -> Option<usize>
where
    F: Fn(&T) -> f32,
{
    if values.is_empty() {
        return None;
    }

    let mut left = 0;
    let mut right = values.len() - 1;
    let mut mid = 0;

    while left < right {
        mid = left + (right - left) / 2;
        match to_f32_func(&values[mid]).partial_cmp(&target)? {
            Ordering::Less => left = mid.checked_add(1).unwrap_or(values.len() - 1),
            Ordering::Greater => right = mid.checked_sub(1).unwrap_or(0),
            Ordering::Equal => return Some(mid),
        }
    }

    let mut closest_index = mid;
    let mut min_diff = (target - to_f32_func(&values[mid])).abs();

    for i in right..=left {
        if let Some(value) = values.get(i).map(|v| to_f32_func(v)) {
            let diff = (target - value).abs();
            if diff <= min_diff {
                closest_index = i;
                min_diff = diff;
            }
        }
    }

    Some(closest_index)
}

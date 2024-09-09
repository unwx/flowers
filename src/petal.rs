use crate::math::{to_cartesian, Rotate};
use glam::{IVec2, Vec2};
use std::f32::consts::PI;

#[derive(Copy, Clone, Debug)]
pub struct PetalPoint {
    pub x: i32,
    pub y: i32,
    pub support: bool,
}

impl PetalPoint {
    pub fn from_ivec2(point: IVec2, support: bool) -> Self {
        PetalPoint {
            x: point.x,
            y: point.y,
            support,
        }
    }
}


pub fn petal_side_sin(k: f32, step: f32, flip: bool, mirror: bool) -> Vec<Vec2> {
    petal_side(k, step, flip, mirror, f32::sin, f32::asin)
}

pub fn petal_side_tan(k: f32, step: f32, flip: bool, mirror: bool) -> Vec<Vec2> {
    petal_side(k, step, flip, mirror, f32::tan, f32::atan)
}


pub fn scale_petal_side(side: &[Vec2], size: u32) -> Vec<IVec2> {
    if side.is_empty() {
        return vec![];
    }

    debug_assert!(size > 0, "Scaling to 0 makes all points equal to 0");
    let scale = |point: Vec2| {
        IVec2::new(
            (point.x * (size as f32)).round() as i32,
            (point.y * (size as f32)).round() as i32,
        )
    };

    let mut scaled_side = Vec::new();
    scaled_side.push(scale(side[0]));

    for i in 1..side.len() {
        let point = side[i];
        let scaled_point = scale(point);
        let previous_scaled_point = *scaled_side.last().unwrap();

        let diff = scaled_point - previous_scaled_point;
        let steps = diff.x.abs().max(diff.y.abs());

        for step in 1..=steps {
            let progress = step as f32 / steps as f32;
            let x = previous_scaled_point.x + (diff.x as f32 * progress).round() as i32;
            let y = previous_scaled_point.y + (diff.y as f32 * progress).round() as i32;
            scaled_side.push(IVec2::new(x, y));
        }
    }

    scaled_side.shrink_to_fit();
    scaled_side
}

pub fn merge_sides(side1: &[IVec2], side2: &[IVec2]) -> Vec<PetalPoint> {
    let mut petal = Vec::with_capacity(side1.len() + side2.len());
    if petal.capacity() == 0 {
        return vec![];
    }

    {
        let non_empty_side = if side1.is_empty() { side2 } else { side1 };
        petal.push(PetalPoint::from_ivec2(*non_empty_side.first().unwrap(), false))
    }

    let mut support_points_on_zero_y = 0;
    let mut last_y_diff = 0;
    let mut last_support_point_index = 0;
    let mut iterator = side1.iter().chain(side2.iter().rev());
    iterator.next();

    for point in iterator {
        let previous_point = petal.last_mut().unwrap();
        let y_diff = point.y - previous_point.y;

        if y_diff != 0 {
            if last_y_diff != y_diff {
                let last_support_point = &mut petal[last_support_point_index];
                last_support_point.support = false;

                if last_support_point.y == 0 {
                    support_points_on_zero_y -= 1;
                }
            }

            petal.push(PetalPoint::from_ivec2(*point, true));
            last_support_point_index = petal.len() - 1;
            last_y_diff = y_diff;

            if point.y == 0 {
                support_points_on_zero_y += 1;
            }
        } else {
            petal.push(PetalPoint::from_ivec2(*point, false));
        }
    }

    if support_points_on_zero_y % 2 == 0 {
        petal.first_mut().unwrap().support = true;
    }

    petal.shrink_to_fit();
    petal
}


pub fn find_petal_range(petal: &[PetalPoint]) -> Vec<(i32, Vec<(i32, i32)>)> {
    if petal.is_empty() {
        return vec![];
    }

    let min_y = petal.iter().min_by_key(|point| point.y).unwrap().y;
    let max_y = petal.iter().max_by_key(|point| point.y).unwrap().y;

    let mut y_to_x_points = Vec::<Vec<i32>>::with_capacity((max_y - min_y) as usize + 1);
    y_to_x_points.resize_with(y_to_x_points.capacity(), || vec![]);

    petal.iter()
        .filter(|point| point.support)
        .for_each(|point| y_to_x_points[(point.y - min_y) as usize].push(point.x));

    for x_points in y_to_x_points.iter_mut() {
        x_points.sort_unstable();
    }

    let mut ranges = Vec::with_capacity(y_to_x_points.len());
    for i in 0..y_to_x_points.len() {
        let x_points = &y_to_x_points[i];
        let y = (i as i32) + min_y;

        if x_points.len() <= 1 {
            ranges.push((y, vec![]));
            continue;
        }

        let mut range = Vec::with_capacity((x_points.len() / 2) + 1);
        {
            let mut index = 0;
            while index < x_points.len() - 1 {
                range.push((x_points[index], x_points[index + 1]));
                index += 2;
            }
        }

        if x_points.len() % 2 != 0 {
            range.push((x_points[x_points.len() - 2], x_points[x_points.len() - 1]))
        }

        ranges.push((y, range));
    }

    ranges
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

    if *petal.first().unwrap() != Vec2::ZERO {
        petal.push(Vec2::ZERO);
    }
    if petal.last().unwrap().y != -1.0 {
        petal.push(Vec2::new(0.0, -1.0));
    }
    if mirror {
        for i in 0..petal.len() {
            petal[i].x = -petal[i].x
        }
    }

    petal
}

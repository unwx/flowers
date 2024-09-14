use crate::math::{to_cartesian, Rotate};
use glam::{IVec2, Vec2};
use std::f32::consts::PI;

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

pub fn merge_sides(side1: &[IVec2], side2: &[IVec2]) -> Vec<(i32, Vec<(i32, bool)>)> {
    let mut petal = Vec::with_capacity(side1.len() + side2.len());
    if petal.capacity() == 0 {
        return vec![];
    }

    struct PetalPoint {
        x: i32,
        y: i32,
        support: bool,
    }
    impl PetalPoint {
        fn from_ivec2(vec: IVec2, support: bool) -> Self {
            Self {
                x: vec.x,
                y: vec.y,
                support,
            }
        }
    }

    {
        let non_empty_side = if side1.is_empty() { side2 } else { side1 };
        petal.push(PetalPoint::from_ivec2(*non_empty_side.first().unwrap(), false))
    }

    {
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
                }

                petal.push(PetalPoint::from_ivec2(*point, true));
                last_support_point_index = petal.len() - 1;
                last_y_diff = y_diff;
            } else {
                petal.push(PetalPoint::from_ivec2(*point, false));
            }
        }
    }

    let y_to_x_petal_points = {
        let (min_y, max_y, support_points_on_zero_y) = {
            let mut min_y = i32::MAX;
            let mut max_y = i32::MIN;
            let mut support_points_on_zero_y = 0u32;

            for point in &petal {
                if min_y > point.y { min_y = point.y }
                if max_y < point.y { max_y = point.y }
                if point.y == 0 && point.support { support_points_on_zero_y += 1 }
            }

            (min_y, max_y, support_points_on_zero_y)
        };

        if support_points_on_zero_y % 2 != 0 {
            petal.first_mut().unwrap().support = true;
        }

        let mut y_to_x_petal_point = Vec::<(i32, Vec<(i32, bool)>)>::with_capacity((max_y - min_y) as usize + 1);

        for i in 0..y_to_x_petal_point.capacity() {
            y_to_x_petal_point.push((min_y + i as i32, vec![]))
        }
        for point in petal {
            y_to_x_petal_point[(point.y - min_y) as usize].1.push((point.x, point.support))
        }

        for x_points in y_to_x_petal_point.iter_mut() {
            x_points.1.sort_unstable_by_key(|point| point.0);
            x_points.1.shrink_to_fit();
        }

        fn remove_extra_last_points(y_to_x_petal_point: &mut Vec<(i32, Vec<(i32, bool)>)>, inverse: bool) {
            let from_index = if inverse { (y_to_x_petal_point.len() - 1)  as i32 } else { 0 };
            let to_index = if inverse { -1 } else { y_to_x_petal_point.len()  as i32 };
            let index_acc = if inverse { -1 } else { 1 };
            let mut index = from_index;
            let mut last_removed_x = 0;

            fn all_x_the_same(x_array: &Vec<(i32, bool)>) -> bool {
                if x_array.is_empty() {
                    return false;
                }

                let last_x = x_array[0].0;
                for i in 1..x_array.len() {
                    if x_array[i].0 != last_x {
                        return false;
                    }
                }

                true
            }

            while index + index_acc != to_index {
                let x_points = &y_to_x_petal_point[index as usize].1;
                if x_points.len() <= 1 || all_x_the_same(x_points) {
                    if !x_points.is_empty() {
                        last_removed_x = x_points[0].0;
                    }

                    y_to_x_petal_point[index as usize].1.clear();
                    index += index_acc;
                    continue;
                }

                let mut last_x = x_points[0].0;
                for i in 1..x_points.len() {
                    let (x, _) = x_points[i];
                    if x != last_x + 1 {
                        if index != from_index {
                            y_to_x_petal_point[(index - index_acc) as usize].1.push((last_removed_x, true));
                            break;
                        }
                    }
                    last_x = x;
                }

                break;
            }
        }

        remove_extra_last_points(&mut y_to_x_petal_point, false);
        remove_extra_last_points(&mut y_to_x_petal_point, true);

        y_to_x_petal_point
    };

    y_to_x_petal_points
}


pub fn find_petal_range(petal: &[(i32, &[(i32, bool)])]) -> Vec<(i32, Vec<(i32, i32)>)> {
    if petal.is_empty() {
        return vec![];
    }

    let mut ranges = Vec::with_capacity(petal.len());
    for y_to_x_points in petal {
        let y = y_to_x_points.0;
        let support_points: Vec<(i32)> = y_to_x_points
            .1
            .iter()
            .filter(|point| point.1)
            .map(|point| point.0)
            .collect();

        if support_points.len() <= 1 {
            ranges.push((y, vec![]));
            continue;
        }

        let mut range = Vec::with_capacity((support_points.len() / 2) + 1);
        {
            let mut index = 0;
            while index < support_points.len() - 1 {
                range.push((support_points[index], support_points[index + 1]));
                index += 2;
            }
        }

        if support_points.len() % 2 != 0 {
            range.push((support_points[support_points.len() - 2], support_points[support_points.len() - 1]))
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

    {
        let control_y = if flip { 1.0 } else { -1.0 };
        if petal.last().unwrap().y != control_y {
            petal.push(Vec2::new(0.0, control_y));
        }
    }
    if mirror {
        for i in 0..petal.len() {
            petal[i].x = -petal[i].x
        }
    }

    petal
}

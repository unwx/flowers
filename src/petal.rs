use crate::math::to_cartesian;
use glam::{Affine2, I16Vec2, Vec2};
use std::f32::consts::PI;
use std::iter;

#[derive(Copy, Clone)]
pub enum MergeMode {
    SideWithSide,
    SideWithOrigin,
}

pub fn side_sin(k: f32, step: f32, rotation: f32, mirror: bool) -> Vec<Vec2> {
    side(k, step, rotation, mirror, f32::sin, f32::asin)
}

pub fn side_tan(k: f32, step: f32, rotation: f32, mirror: bool) -> Vec<Vec2> {
    side(k, step, rotation, mirror, f32::tan, f32::atan)
}

pub fn scale_and_merge_sides(
    side1: &[Vec2],
    side2: &[Vec2],
    size: u16,
    merge_mode: MergeMode,
) -> Vec<I16Vec2> {
    assert!(!side1.is_empty(), "side1 cannot be empty");
    assert!(!side2.is_empty(), "side2 cannot be empty");
    assert!(
        size > 1 && i16::try_from(size).is_ok(),
        "illegal size '{size}', allowed: [1 < size <= 32_767]"
    );

    let mut petal_frame = Vec::with_capacity((size as usize) * 3);
    let scale = |point: Vec2| -> I16Vec2 {
        debug_assert!(point.x <= 1.0 && point.y <= 1.0);
        let scaled = (point * f32::from(size)).round();
        I16Vec2::new(scaled.x as i16, scaled.y as i16)
    };

    let mut iterator: Box<dyn Iterator<Item = &Vec2>> = match merge_mode {
        MergeMode::SideWithSide => Box::new(side1.iter().chain(side2.iter().rev())),
        MergeMode::SideWithOrigin => Box::new(
            side1
                .iter()
                .chain(iter::once(&Vec2::ZERO))
                .chain(side2.iter())
                .chain(iter::once(&Vec2::ZERO)),
        ),
    };

    petal_frame.push(scale(*iterator.next().unwrap()));
    for point in iterator {
        let scaled_point = scale(*point);
        let previous_scaled_point = *petal_frame.last().unwrap();

        let diff = {
            let i16_diff = scaled_point - previous_scaled_point;
            Vec2::new(f32::from(i16_diff.x), f32::from(i16_diff.y))
        };
        let steps = diff.x.abs().max(diff.y.abs());
        let mut step = 1.0;

        while step <= steps {
            let progress = step / steps;
            let interpolated_point = I16Vec2::new(
                previous_scaled_point.x + (diff.x * progress).round() as i16,
                previous_scaled_point.y + (diff.y * progress).round() as i16,
            );

            if *petal_frame.last().unwrap() != interpolated_point {
                petal_frame.push(interpolated_point);
            }

            step += 1.0;
        }
    }

    petal_frame.shrink_to_fit();
    petal_frame
}

pub fn find_petal_area(petal: &[I16Vec2]) -> Vec<(I16Vec2, I16Vec2)> {
    if petal.len() <= 1 {
        return vec![];
    }

    let min_y = petal.iter().min_by_key(|point| point.y).unwrap().y;
    let max_y = petal.iter().max_by_key(|point| point.y).unwrap().y;
    debug_assert!(min_y <= max_y);

    let mut checkpoints: Vec<Vec<i16>> = Vec::with_capacity((max_y - min_y) as usize + 1);
    checkpoints.resize_with(checkpoints.capacity(), Vec::new);

    {
        let mut last_y_diff = 0;

        for i in 1..petal.len() {
            let point = petal[i];
            let previous_point = petal[i - 1];
            let y_diff = point.y - previous_point.y;

            if y_diff == 0 {
                continue;
            }

            if last_y_diff != y_diff {
                let index = (previous_point.y - min_y) as usize;
                if let Some(last_checkpoint) = checkpoints[index].last() {
                    let last = *last_checkpoint;
                    checkpoints[index].push(last);
                }

                last_y_diff = y_diff;
            }

            checkpoints[(point.y - min_y) as usize].push(point.x);
        }
    }

    {
        let index = (0 - min_y) as usize;
        if checkpoints[index].len() % 2 != 0 {
            checkpoints[index].push(0);
        }
    }

    for x_points in &mut checkpoints {
        x_points.sort_unstable();
    }

    let mut area = Vec::with_capacity(checkpoints.len() * 2);
    {
        let sorted_petal_frame = {
            let mut frame: Vec<Vec<i16>> = Vec::with_capacity((max_y - min_y) as usize + 1);
            frame.resize_with(frame.capacity(), || vec![]);

            for point in petal {
                frame[(point.y - min_y) as usize].push(point.x);
            }
            for vec in &mut frame {
                vec.sort_unstable();
            }

            frame
        };

        let mut push_range = |x1: i16, x2: i16, y: i16, frame_line_index: usize| -> Option<usize> {
            if x2 - x1 <= 1 {
                return None;
            }

            let frame_line = &sorted_petal_frame[(y - min_y) as usize];
            let find_x_index = |x_to_find: i16, from_index: usize| -> Option<usize> {
                for i in from_index..frame_line.len() {
                    if frame_line[i] == x_to_find {
                        return Some(i);
                    }
                }

                return None;
            };

            let x1_index = find_x_index(x1, frame_line_index)?;
            let x2_index = find_x_index(x2, x1_index)?;

            let mut actual_x1 = x1;
            let mut actual_x2 = x2;

            for i in (x1_index + 1)..x2_index {
                let x = frame_line[i];
                debug_assert!(x >= actual_x1);

                if x - actual_x1 > 1 {
                    break;
                }

                actual_x1 = x;
            }
            if x2_index != 0 {
                let mut i = x2_index - 1;
                while i > x1_index {
                    let x = frame_line[i];
                    debug_assert!(actual_x2 >= x);

                    if actual_x2 - x > 1 {
                        break;
                    }

                    actual_x2 = x;
                    i -= 1;
                }
            }

            if actual_x2 - actual_x1 > 1 {
                area.push((
                    I16Vec2::new(actual_x1 + 1, y),
                    I16Vec2::new(actual_x2 - 1, y),
                ));
            }

            Some(x2_index)
        };

        for (checkpoint_y_index, x_checkpoints) in checkpoints.iter().enumerate() {
            let y = min_y + (checkpoint_y_index as i16);
            if x_checkpoints.len() <= 1 {
                continue;
            }

            let mut checkpoint_index = 0;
            let mut frame_line_last_index = 0;

            while checkpoint_index < x_checkpoints.len() - 1 {
                let last_index = push_range(
                    x_checkpoints[checkpoint_index],
                    x_checkpoints[checkpoint_index + 1],
                    y,
                    frame_line_last_index,
                );

                frame_line_last_index = last_index.unwrap_or(frame_line_last_index);
                checkpoint_index += 2;
            }

            if x_checkpoints.len() % 2 != 0 {
                push_range(
                    x_checkpoints[x_checkpoints.len() - 2],
                    x_checkpoints[x_checkpoints.len() - 1],
                    y,
                    0,
                );
            }
        }
    }

    area.shrink_to_fit();
    area
}

fn side<Func, ArcFunc>(
    k: f32,
    step: f32,
    rotation: f32,
    mirror: bool,
    trig_func: Func,
    arc_trig_func: ArcFunc,
) -> Vec<Vec2>
where
    Func: Fn(f32) -> f32,
    ArcFunc: Fn(f32) -> f32,
{
    let mut side = Vec::with_capacity(((arc_trig_func(1.0) / k) / step) as usize + 1);
    if side.capacity() == 0 {
        return side;
    }

    for i in 0..side.capacity() {
        let theta = (i as f32) * step;
        side.push(to_cartesian(trig_func(theta * k), theta));
    }

    {
        let rotation = rotation + side.last().unwrap().angle_to(Vec2::Y) + PI;
        let affine = Affine2::from_angle(rotation);

        for point in &mut side {
            *point = affine.transform_vector2(*point);
        }
    }

    if mirror {
        for point in &mut side {
            point.x = -point.x;
        }
    }

    side
}

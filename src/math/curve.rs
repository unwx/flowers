use crate::math::real::{debug_assert_finite, debug_eval_finite};
use crate::math::to_cartesian;
use glam::{I16Vec2, Mat2, Vec2};
use std::iter::Rev;

#[derive(Copy, Clone)]
pub enum MergeMode {
    ZigZag,
    Origin(I16Vec2),
}

#[must_use]
pub fn merge(curves: &[&[I16Vec2]], mode: MergeMode) -> Vec<I16Vec2> {
    if curves.is_empty() {
        return vec![];
    }

    let mut merged = Vec::with_capacity({
        let length = curves.iter().map(|c| c.len()).sum();
        match mode {
            MergeMode::ZigZag => length,
            MergeMode::Origin(_) => length + curves.len(),
        }
    });

    match mode {
        MergeMode::ZigZag => {
            let mut head = true;

            for curve in curves {
                enum DynIterator<'a> {
                    Basic(std::slice::Iter<'a, I16Vec2>),
                    Reverse(Rev<std::slice::Iter<'a, I16Vec2>>),
                }
                impl<'a> Iterator for DynIterator<'a> {
                    type Item = &'a I16Vec2;

                    fn next(&mut self) -> Option<Self::Item> {
                        match self {
                            DynIterator::Basic(i) => i.next(),
                            DynIterator::Reverse(i) => i.next(),
                        }
                    }
                }

                let iterator = if head {
                    DynIterator::Basic(curve.iter())
                } else {
                    DynIterator::Reverse(curve.iter().rev())
                };

                for point in iterator {
                    merged.push(*point);
                }

                head = !head;
            }
        }
        MergeMode::Origin(origin) => {
            for curve in curves {
                for point in curve.iter() {
                    merged.push(*point);
                }

                merged.push(origin);
            }
        }
    }

    merged
}

#[must_use]
pub fn scale(curve: &[Vec2], factor: u16) -> Vec<I16Vec2> {
    if curve.is_empty() {
        return vec![];
    }
    assert!(factor >= 1, "factor must be >= 1");
    debug_assert_finite!(curve);

    let scale = |point: Vec2| -> I16Vec2 {
        debug_eval_finite!((point * factor as f32).round()).as_i16vec2()
    };
    let mut scaled_curve = Vec::with_capacity(curve.len());
    scaled_curve.push(scale(curve[0]));

    for point in curve.iter().skip(1) {
        let scaled_point = scale(*point);
        if *scaled_curve.last().unwrap() != scaled_point {
            scaled_curve.push(scaled_point);
        }
    }

    scaled_curve.shrink_to_fit();
    scaled_curve
}

#[must_use]
pub fn eval_polar_sin(k: f32, step: f32, angle: f32, mirror: bool) -> Vec<Vec2> {
    eval_polar(k, step, angle, mirror, f32::sin, f32::asin)
}

#[must_use]
pub fn eval_polar_tan(k: f32, step: f32, angle: f32, mirror: bool) -> Vec<Vec2> {
    eval_polar(k, step, angle, mirror, f32::tan, f32::atan)
}

#[must_use]
fn eval_polar<Func, ArcFunc>(
    k: f32,
    step: f32,
    angle: f32,
    mirror: bool,
    trig_func: Func,
    arc_trig_func: ArcFunc,
) -> Vec<Vec2>
where
    Func: Fn(f32) -> f32,
    ArcFunc: Fn(f32) -> f32,
{
    debug_assert_finite!(k, step, angle);
    assert!(k > 0.0, "k must be > 0.0");
    assert!(step > 0.0, "step must be > 0.0");

    let length = {
        let float = debug_eval_finite!((arc_trig_func(1.0) / k) / step).max(0.0);
        if float as f64 > usize::MAX as f64 {
            panic!(
                "Polar function visualization length exceeds usize::MAX. \
                Consider adjusting parameters, especially 'step'. \
                Current parameters: [k: {k}, step: {step}, angle: {angle}, mirror: {mirror}]"
            )
        }
        float as usize
    };

    if length == 0 {
        return vec![];
    }

    let mut curve = Vec::with_capacity(length);
    for i in 0..length {
        let theta = i as f32 * step;
        let point = to_cartesian(trig_func(theta * k), theta);

        debug_assert_finite!(point);
        curve.push(point);
    }

    {
        let last_point_angle = curve.last().unwrap().to_angle();
        debug_assert_finite!(last_point_angle);

        let rotation = Mat2::from_angle(angle - last_point_angle);
        for point in &mut curve {
            *point = rotation.mul_vec2(*point);
        }
    }

    if mirror {
        for point in &mut curve {
            point.y = -point.y;
        }
    }

    debug_assert_eq!(curve.len(), length);
    debug_assert_finite!(&curve);
    curve
}

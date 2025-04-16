use crate::math::definition::{Point, Scalar};
use crate::util::macros::{debug_assert_finite, debug_eval_finite};
use glam::Mat2;

pub fn to_cartesian(radius: Scalar, theta: Scalar) -> Point {
    debug_assert_finite!(radius, theta);
    debug_eval_finite!(Point::new(radius * theta.cos(), radius * theta.sin()))
}


pub fn eval_polar_sin(k: Scalar, step: Scalar, angle: Scalar, mirror: bool) -> Vec<Point> {
    eval_polar(k, step, angle, mirror, Scalar::sin, Scalar::asin)
}

pub fn eval_polar_tan(k: Scalar, step: Scalar, angle: Scalar, mirror: bool) -> Vec<Point> {
    eval_polar(k, step, angle, mirror, Scalar::tan, Scalar::atan)
}

fn eval_polar<Func, ArcFunc>(
    k: Scalar,
    step: Scalar,
    angle: Scalar,
    mirror: bool,
    trig_func: Func,
    arc_trig_func: ArcFunc,
) -> Vec<Point>
where
    Func: Fn(Scalar) -> Scalar,
    ArcFunc: Fn(Scalar) -> Scalar,
{
    debug_assert_finite!(k, step, angle);
    assert!(k > 0.0, "k must be > 0.0");
    assert!(step > 0.0, "step must be > 0.0");

    let length = {
        let length = (arc_trig_func(1.0) / k) / step;
        debug_assert_finite!(length);

        if length.is_finite() {
            length.clamp(0.0, usize::MAX as Scalar) as usize
        } else {
            0
        }
    };

    if length == 0 {
        return vec![];
    }


    let mut curve: Vec<Point> = (0..length)
        .map(|i| {
            let theta = i as Scalar * step;
            to_cartesian(trig_func(theta * k), theta)
        })
        .collect();

    {
        let last_point_angle = curve
            .last()
            .map(|point| debug_eval_finite!(point.to_angle()))
            .expect("curve cannot be empty");

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

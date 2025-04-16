use crate::math::definition::{IntPoint, Point, PointExtensions, Scalar};
use crate::util::macros::{debug_assert_finite, debug_assert_interpolated, debug_eval_finite};
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Deref;

// TODO Docs
//  - A ClosedCurve cannot be empty.
//  - The first and last points of a ClosedCurve are always the same (the curve is self-closed).
//  - A ClosedCurve should be interpolated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedCurve {
    points: Vec<IntPoint>,
}

impl TryFrom<Vec<IntPoint>> for ClosedCurve {
    type Error = ();

    fn try_from(points: Vec<IntPoint>) -> Result<Self, Self::Error> {
        if !points.is_empty() && points.first() == points.last() {
            debug_assert_interpolated!(&points);
            Ok(ClosedCurve { points })
        } else {
            Err(())
        }
    }
}

impl Deref for ClosedCurve {
    type Target = Vec<IntPoint>;

    fn deref(&self) -> &Self::Target {
        &self.points
    }
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum MergeMode {
    ZigZag,
    Origin,
}

pub fn merge<T: Clone>(curves: Vec<Vec<T>>, mode: MergeMode) -> Vec<T> {
    let capacity = {
        let length = curves.iter().map(|c| c.len()).sum();
        match mode {
            MergeMode::ZigZag => length,
            MergeMode::Origin => length + curves.len(),
        }
    };
    let mut merged = Vec::with_capacity(capacity);

    match mode {
        MergeMode::ZigZag => {
            let mut forward = true;

            for curve in curves {
                let iterator: Box<dyn Iterator<Item = T>> = if forward {
                    Box::new(curve.into_iter())
                } else {
                    Box::new(curve.into_iter().rev())
                };

                merged.extend(iterator);
                forward = !forward;
            }
        }
        MergeMode::Origin => {
            for curve in curves {
                if let Some(origin) = curve.first().cloned() {
                    merged.extend(curve.into_iter());
                    merged.push(origin);
                }
            }
        }
    }

    debug_assert_eq!(merged.len(), capacity);
    merged
}


pub fn scale(curve: &[Point], factor: u16) -> Vec<IntPoint> {
    debug_assert_finite!(curve);
    let factor = factor as Scalar;

    let mut scaled_curve = Vec::with_capacity(curve.len());
    for scaled_point in curve
        .iter()
        .map(|p| debug_eval_finite!((p * factor).round()).as_int_point())
    {
        if Some(&scaled_point) != scaled_curve.last() {
            scaled_curve.push(scaled_point);
        }
    }

    scaled_curve.shrink_to_fit();
    scaled_curve
}

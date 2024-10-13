use colorgrad::Color;
use glam::Vec2;
use palette::rgb::Rgb;
use palette::{Hsl, RgbHue};
use std::any::type_name;

/*
 * IsFinite
 */

pub trait IsFinite
where
    Self: Clone,
{
    #[allow(clippy::wrong_self_convention)]
    fn is_finite(self) -> bool;
}

impl<T: IsFinite> IsFinite for &T {
    fn is_finite(self) -> bool {
        self.clone().is_finite()
    }
}

impl IsFinite for f32 {
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }
}

impl IsFinite for f64 {
    fn is_finite(self) -> bool {
        f64::is_finite(self)
    }
}

impl IsFinite for Vec2 {
    fn is_finite(self) -> bool {
        Vec2::is_finite(self)
    }
}

impl<S, T: IsFinite + From<RgbHue<T>>> IsFinite for Hsl<S, T> {
    fn is_finite(self) -> bool {
        T::from(self.hue).is_finite() && self.saturation.is_finite() && self.lightness.is_finite()
    }
}

impl<S, T: IsFinite> IsFinite for Rgb<S, T> {
    fn is_finite(self) -> bool {
        self.red.is_finite() && self.green.is_finite() && self.blue.is_finite()
    }
}

impl IsFinite for Color {
    fn is_finite(self) -> bool {
        self.r.is_finite() && self.g.is_finite() && self.b.is_finite()
    }
}

/*
 * FiniteChecker
 */

pub(crate) trait FiniteChecker {
    fn debug_assert_finite(self, expression: &'static str);
}

fn debug_assert_finite_template<T>(value: T, expression: &'static str)
where
    T: IsFinite,
{
    debug_assert!(
        value.is_finite(),
        "value was not finite. [expression: '{}', type: {}]",
        expression,
        type_name::<T>()
    )
}

impl<T: IsFinite> FiniteChecker for T {
    fn debug_assert_finite(self, expression: &'static str) {
        debug_assert_finite_template::<T>(self, expression);
    }
}

impl<T: IsFinite> FiniteChecker for &[T] {
    fn debug_assert_finite(self, expression: &'static str) {
        for value in self {
            debug_assert_finite_template::<&T>(value, expression);
        }
    }
}

impl<T: IsFinite> FiniteChecker for &Vec<T> {
    fn debug_assert_finite(self, expression: &'static str) {
        self.as_slice().debug_assert_finite(expression);
    }
}

macro_rules! debug_eval_finite {
    ($e:expr) => {{
        let result = $e;
        crate::math::real::FiniteChecker::debug_assert_finite(result, stringify!($e));
        result
    }};
}

macro_rules! debug_assert_finite {
    ($($expr:expr),+ $(,)?) => {
        $(
            crate::math::real::FiniteChecker::debug_assert_finite($expr, stringify!($expr));
        )+
    };
}

pub(crate) use debug_assert_finite;
pub(crate) use debug_eval_finite;

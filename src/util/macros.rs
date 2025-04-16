use crate::math::definition::{IntPoint, IntPointExtensions};
use crate::math::real::IsFinite;
use std::any::type_name;

macro_rules! debug_eval_finite {
    ($expr:expr) => {{
        let result = $expr;
        if cfg!(debug_assertions) {
            $crate::util::macros::Assert::new(result, stringify!($expr)).finite()
        }

        result
    }};
}

macro_rules! debug_assert_finite {
    ($($expr:expr),+ $(,)?) => {
        if cfg!(debug_assertions) {
            $(
                $crate::util::macros::Assert::new($expr, stringify!($expr)).finite();
            )+
        }
    };
}

macro_rules! debug_assert_interpolated {
    ($($expr:expr),+ $(,)?) => {
        if cfg!(debug_assertions) {
            $(
                $crate::util::macros::Assert::new($expr, stringify!($expr)).interpolated();
            )+
        }
    };
}


pub struct Assert<T> {
    value: T,
    expression: &'static str,
}

impl<T> Assert<T> {
    pub fn new(value: T, expression: &'static str) -> Self {
        Self { value, expression }
    }

    pub fn transform<F, NT>(self, func: F) -> Assert<NT>
    where
        F: FnOnce(T) -> NT,
    {
        Assert::new(func(self.value), self.expression)
    }
}


impl<T: IsFinite> Assert<T> {
    pub fn finite(self) {
        assert!(
            self.value.is_finite(),
            "value is not finite. [expression: '{}', type: '{}']",
            self.expression,
            type_name::<T>()
        )
    }
}

impl<T: IsFinite> Assert<&[T]> {
    pub fn finite(self) {
        for (index, value) in self.value.iter().enumerate() {
            assert!(
                value.is_finite(),
                "array[{}] is not finite. [expression: '{}', type: '{}']",
                index,
                self.expression,
                type_name::<T>()
            );
        }
    }
}

impl<T: IsFinite> Assert<&Vec<T>> {
    pub fn finite(self) {
        self.transform(|value| value.as_slice()).finite()
    }
}


impl Assert<&[IntPoint]> {
    pub fn interpolated(self) {
        for i in 1..self.value.len() {
            let past = self.value[i - 1];
            let current = self.value[i];

            assert!(
                past.is_near_to(current),
                "points array is not interpolated at [{}..={}]. \
                [past_point: {}, current_point: {} expression: '{}', type: '{}']",
                i - 1,
                i,
                past,
                current,
                self.expression,
                type_name::<IntPoint>()
            );
        }
    }
}

impl Assert<&Vec<IntPoint>> {
    pub fn interpolated(self) {
        self.transform(|value| value.as_slice()).interpolated()
    }
}


pub(crate) use debug_assert_finite;
pub(crate) use debug_assert_interpolated;
pub(crate) use debug_eval_finite;

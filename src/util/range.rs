use rand::Rng;
use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub struct RandomF32RangeFactory {
    boundaries: RangeInclusive<f32>,
    min_length: f32,
    max_length: f32,
}

impl RandomF32RangeFactory {
    pub fn new(boundaries: RangeInclusive<f32>, min_length: f32, max_length: f32) -> Self {
        assert!(!boundaries.is_empty(), "boundaries is empty");
        assert!(min_length >= 0.0, "min_length must be >= 0.0");
        assert!(max_length >= 0.0, "max_length must be >= 0.0");
        assert!(
            min_length <= max_length,
            "min_length({min_length}) must be <= max_length({max_length})"
        );

        let available_length = *boundaries.end() - *boundaries.start();
        Self {
            boundaries,
            min_length: min_length.min(available_length),
            max_length: max_length.min(available_length),
        }
    }

    pub fn boundaries(&self) -> RangeInclusive<f32> {
        self.boundaries.clone()
    }

    pub fn min_length(&self) -> f32 {
        self.min_length
    }

    pub fn max_length(&self) -> f32 {
        self.max_length
    }


    pub fn random_range<R: Rng>(&self, random: &mut R) -> RangeInclusive<f32> {
        let start = {
            let from = *self.boundaries.start();
            let to = *self.boundaries.end() - self.min_length;
            random.gen_range(from..=to)
        };
        let end = {
            let from = start + self.min_length;
            let to = (start + self.max_length).min(*self.boundaries.end());
            random.gen_range(from..=to)
        };

        start..=end
    }
}


pub fn is_range_within_another<T>(range: RangeInclusive<T>, another: RangeInclusive<T>) -> bool
where
    T: PartialOrd + Copy,
{
    is_range_within_another_ref(&range, &another)
}

pub fn is_range_within_another_ref<T: PartialOrd>(
    range: &RangeInclusive<T>,
    another: &RangeInclusive<T>,
) -> bool {
    another.contains(range.start()) && another.contains(range.end())
}

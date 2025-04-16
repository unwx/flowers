use crate::math::definition::Int;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Range {
    /// Inclusive.
    from: Int,

    /// Inclusive.
    to: Int,
}

impl Range {
    pub fn new(from: Int, to: Int) -> Self {
        Self::try_new(from, to).unwrap_or_else(|| panic!("from({from}) must be <= to({to})"))
    }

    pub fn try_new(from: Int, to: Int) -> Option<Self> {
        if from <= to {
            Some(Self { from, to })
        } else {
            None
        }
    }


    pub fn from(self) -> Int {
        self.from
    }

    pub fn to(self) -> Int {
        self.to
    }


    pub fn iter(self) -> impl Iterator<Item = Int> {
        self.from..=self.to
    }

    pub fn coverage(self) -> usize {
        (self.to - self.from) as usize + 1
    }

    pub fn intersects(self, other: Self) -> bool {
        self.from <= other.to && other.from <= self.to
    }
}

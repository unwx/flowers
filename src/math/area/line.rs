use crate::math::area::range::Range;
use crate::math::definition::Int;
use std::ops::Deref;

// TODO Docs
//  Line guarantees:
//  1. Not empty.
//  2. Sorted in ascending order.
//  3. No overlapping ranges.
//  4. No adjacent ranges (i.e., `(ranges[i].from - ranges[i - 1].to) >= 2` for all valid `i`).
#[derive(Debug, Clone)]
pub struct Line {
    ranges: Vec<Range>,
    min_x: Int,
    max_x: Int,
    coverage: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LineError {
    Empty,
    Invalid,
}

impl Line {
    pub fn ranges(&self) -> &Vec<Range> {
        &self.ranges
    }

    pub fn into_ranges(self) -> Vec<Range> {
        self.ranges
    }

    pub fn min_x(&self) -> Int {
        self.min_x
    }

    pub fn max_x(&self) -> Int {
        self.max_x
    }

    pub fn coverage(&self) -> usize {
        self.coverage
    }

    pub fn flatten(&self) -> impl Iterator<Item = Int> + '_ {
        self.ranges
            .iter()
            .flat_map(|range| range.from()..=range.to())
    }


    // TODO Docs
    //  Faster than `intersects`.
    //  May produce false-positives, but never false-negatives.
    //
    // TODO Tests
    pub fn dirty_intersects(&self, other: &Line) -> bool {
        self.max_x() >= other.min_x() && other.max_x() >= self.min_x()
    }

    pub fn intersects(&self, other: &Line) -> bool {
        if !self.dirty_intersects(other) {
            return false;
        }

        let mut self_range_index = 0;
        let mut other_range_index = 0;

        while self_range_index < self.ranges.len() && other_range_index < other.ranges.len() {
            let self_range = self.ranges[self_range_index];
            let other_range = other.ranges[other_range_index];

            if self_range.from() > other_range.to() {
                other_range_index += 1;
                continue;
            }
            if other_range.from() > self_range.to() {
                self_range_index += 1;
                continue;
            }

            return true;
        }

        false
    }


    // TODO Tests
    pub fn query_slice(&self, query: Range) -> Option<&[Range]> {
        if query.from() > self.max_x() || query.to() < self.min_x() {
            return None;
        }

        let intersects = |range_index: Option<usize>| -> bool {
            range_index
                .and_then(|it| self.ranges.get(it))
                .map(|range| range.intersects(query))
                .unwrap_or(false)
        };

        let start_index = match self
            .ranges
            .binary_search_by_key(&query.from(), |range| range.from())
        {
            Ok(index) => Some(index),

            // Visualization
            //
            // query: 5..=15
            // ranges: [1..=1], [3..=8], [10..=10], [25..=30]
            //
            // ↓ (binary_search_by_key returns Err(2))
            //
            // failure_index: 2
            // ranges[failure_index - 1](3..=8) intersects query(5..=15),
            // so `failure_index - 1`(1) is the index of the first relevant range.

            // Visualization
            //
            // query: 5..=15
            // ranges: [1..=1], [3..=4], [6..=10], [25..=30]
            //
            // ↓ (binary_search_by_key returns Err(2))
            //
            // failure_index: 2
            // ranges[failure_index - 1](3..=4) does not intersect query(5..=15),
            // so failure_index(2) points to the first range that might start after the query.

            // Explanation:
            // When binary_search_by_key returns Err(failure_index), it means query.from might be inserted
            // at failure_index to maintain sorted order.
            //
            // We only need to check one range behind due to the Line guarantees.
            // If ranges[failure_index - 1] doesn't intersect, any range before it won't either.
            // Similarly, if ranges[failure_index] doesn't intersect, any range after it won't be a starting point.
            Err(failure_index) => [failure_index.checked_sub(1), Some(failure_index)]
                .into_iter()
                .find(|&index| intersects(index))
                .flatten(),
        }?;

        let end_index = match self
            .ranges
            .binary_search_by_key(&query.to(), |range| range.to())
        {
            Ok(index) => Some(index),

            // Visualization
            //
            // query: 5..=15
            // ranges: [3..=4], [6..=10], [12..=16], [35..=36]
            //
            // ↓ (binary_search_by_key returns Err(2))
            //
            // failure_index: 2
            // ranges[failure_index](12..=16) intersects query(5..=15),
            // so failure_index(2) is the index of the last relevant range.

            // Visualization
            //
            // query: 5..=15
            // ranges: [3..=4], [6..=10], [25..=30], [35..=36]
            //
            // ↓ (binary_search_by_key returns Err(2))
            //
            // failure_index: 2
            // ranges[failure_index](25..=30) does not intersect query(5..=15),
            // so `failure_index - 1`(1) points to the last range that might end before or within the query.
            Err(failure_index) => [Some(failure_index), failure_index.checked_sub(1)]
                .into_iter()
                .find(|&index| intersects(index))
                .flatten(),
        }?;

        debug_assert!(start_index <= end_index);
        Some(&self.ranges[start_index..=end_index])
    }
}

// TODO Docs
//  The `Vec<Range>` must comply with the following invariants:
//  1. Ranges are sorted in ascending order by their `from` value.
//  2. Ranges do not overlap.
impl TryFrom<Vec<Range>> for Line {
    type Error = LineError;

    fn try_from(ranges: Vec<Range>) -> Result<Self, Self::Error> {
        if ranges.is_empty() {
            return Err(LineError::Empty);
        }
        if ranges.len() == 1 {
            let range = ranges[0];
            return Ok(Self {
                ranges,
                min_x: range.from(),
                max_x: range.to(),
                coverage: range.coverage(),
            });
        }


        let mut merged_ranges = Vec::with_capacity(ranges.len());
        merged_ranges.push(ranges[0]);

        for current in ranges.into_iter().skip(1) {
            let past = *merged_ranges.last().expect("merged_ranges cannot be empty");

            if past.to() >= current.from() {
                return Err(LineError::Invalid);
            }
            if current.from() - past.to() == 1 {
                *merged_ranges
                    .last_mut()
                    .expect("merged_ranges cannot be empty") =
                    Range::new(past.from(), current.to());
            } else {
                merged_ranges.push(current);
            }
        }

        merged_ranges.shrink_to_fit();
        let min_x = merged_ranges
            .first()
            .expect("merged_ranges cannot be empty")
            .from();
        let max_x = merged_ranges
            .last()
            .expect("merged_ranges cannot be empty")
            .to();
        let coverage = merged_ranges.iter().map(|range| range.coverage()).sum();

        Ok(Self {
            ranges: merged_ranges,
            min_x,
            max_x,
            coverage,
        })
    }
}

impl Deref for Line {
    type Target = Vec<Range>;

    fn deref(&self) -> &Self::Target {
        &self.ranges
    }
}

impl PartialEq for Line {
    fn eq(&self, other: &Self) -> bool {
        self.coverage == other.coverage
            && self.min_x == other.min_x
            && self.max_x == other.max_x
            && self.ranges == other.ranges
    }
}

impl Eq for Line {}

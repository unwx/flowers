use crate::math::curve::ClosedCurve;
use crate::math::definition::Int;
use std::mem;
use std::ops::{Index, IndexMut, RangeInclusive};

#[derive(Debug, Clone)]
pub struct Grid2<L> {
    lines: Vec<L>,
    min_y: Int,
    max_y: Int,
}

impl<L> Grid2<L> {
    pub fn new<F>(min_y: Int, max_y: Int, mut empty_line: F) -> Self
    where
        F: FnMut() -> L,
    {
        assert!(min_y <= max_y, "min_y must be <= max_y");

        let height = Self::height_of(min_y, max_y);
        let mut lines = Vec::with_capacity(height);

        for _ in 0..height {
            lines.push(empty_line());
        }

        Self {
            lines,
            min_y,
            max_y,
        }
    }


    pub fn line(&self, y: Int) -> Option<&L> {
        let index = self.y_to_index_checked(y)?;
        Some(&self.lines[index])
    }

    pub fn line_mut(&mut self, y: Int) -> Option<&mut L> {
        let index = self.y_to_index_checked(y)?;
        Some(&mut self.lines[index])
    }


    pub fn replace_line(&mut self, y: Int, line: L) -> L {
        let index = self.y_to_index(y);
        mem::replace(&mut self.lines[index], line)
    }


    pub fn iter(&self) -> impl Iterator<Item = &L> + '_ {
        self.lines.iter()
    }

    pub fn into_iter(self) -> impl Iterator<Item = L> {
        self.lines.into_iter()
    }

    pub fn enumerate(&self) -> impl Iterator<Item = (Int, &L)> + '_ {
        let min_y = self.min_y;
        let lines_len = self.lines.len();

        self.iter()
            .enumerate()
            .map(move |(index, line)| (Self::index_to_y(index, min_y, lines_len), line))
    }

    pub fn into_enumerate(self) -> impl Iterator<Item = (Int, L)> {
        let min_y = self.min_y;
        let lines_len = self.lines.len();

        self.into_iter()
            .enumerate()
            .map(move |(index, line)| (Self::index_to_y(index, min_y, lines_len), line))
    }


    pub fn shrink_to(&mut self, range: RangeInclusive<Int>) {
        assert!(
            *range.start() >= self.min_y,
            "invalid range: range.start({}) must be >= min_y({})",
            range.start(),
            self.min_y
        );
        assert!(
            *range.end() <= self.max_y,
            "invalid range: range.end({}) must be <= max_y({})",
            range.end(),
            self.max_y
        );

        if self.min_y != *range.start() {
            let to = (*range.start() - self.min_y) as usize;
            self.lines.drain(..to);
            self.min_y = *range.start();
        }
        if self.max_y != *range.end() {
            let from = self.lines.len() - (self.max_y - *range.end()) as usize;
            self.lines.drain(from..);
            self.max_y = *range.end();
        }

        self.lines.shrink_to_fit()
    }

    pub fn transform<NL, F>(self, transform: F) -> Grid2<NL>
    where
        F: FnMut(L) -> NL,
    {
        let min_y = self.min_y;
        let max_y = self.max_y;
        let lines = self.into_iter().map(transform).collect();

        Grid2 {
            lines,
            min_y,
            max_y,
        }
    }


    pub fn min_y(&self) -> Int {
        self.min_y
    }

    pub fn max_y(&self) -> Int {
        self.max_y
    }

    pub fn height(&self) -> usize {
        self.lines.len()
    }


    fn height_of(min_y: Int, max_y: Int) -> usize {
        debug_assert!(min_y <= max_y);
        ((max_y as i128 - min_y as i128) + 1) as usize
    }

    fn index_to_y(index: usize, min_y: Int, lines_len: usize) -> Int {
        debug_assert!(index < lines_len);
        min_y + index as Int
    }

    fn y_to_index(&self, y: Int) -> usize {
        self.y_to_index_checked(y).unwrap_or_else(|| {
            panic!(
                "'y' out of bounds. [y: {}, bounds: [{}..={}]]",
                y, self.min_y, self.max_y
            )
        })
    }

    fn y_to_index_checked(&self, y: Int) -> Option<usize> {
        if y < self.min_y || y > self.max_y {
            return None;
        }

        Some((y - self.min_y) as usize)
    }
}


impl<L> Index<Int> for Grid2<L> {
    type Output = L;

    fn index(&self, y: Int) -> &Self::Output {
        let index = self.y_to_index(y);
        &self.lines[index]
    }
}

impl<L> IndexMut<Int> for Grid2<L> {
    fn index_mut(&mut self, y: Int) -> &mut Self::Output {
        let index = self.y_to_index(y);
        &mut self.lines[index]
    }
}


impl<L: PartialEq> PartialEq for Grid2<L> {
    fn eq(&self, other: &Self) -> bool {
        self.min_y == other.min_y && self.max_y == other.max_y && self.lines == other.lines
    }
}

impl<L: Eq> Eq for Grid2<L> {}


impl<T> Grid2<Vec<T>> {
    pub fn push(&mut self, y: Int, element: T) {
        self[y].push(element);
    }

    pub fn append(&mut self, y: Int, elements: &mut Vec<T>) {
        self[y].append(elements);
    }

    pub fn sort_unstable_by_key<K, F>(&mut self, mut compare: F)
    where
        F: FnMut(&T) -> K,
        K: Ord,
    {
        for line in &mut self.lines {
            line.sort_unstable_by_key(&mut compare);
        }
    }
}

impl<T> Grid2<Vec<T>>
where
    T: Ord,
{
    pub fn sort_unstable(&mut self) {
        for line in &mut self.lines {
            line.sort_unstable();
        }
    }
}


impl<'a> From<&'a ClosedCurve> for Grid2<Vec<Int>> {
    fn from(curve: &'a ClosedCurve) -> Self {
        let min = curve.iter().min_by_key(|point| point.y);
        let max = curve.iter().max_by_key(|point| point.y);

        let (&min, &max) = min
            .zip(max)
            .unwrap_or_else(|| panic!("curve cannot be empty due to its guarantees"));
        let mut grid = Grid2::new(min.y, max.y, Vec::new);

        for point in curve.iter() {
            grid.push(point.y, point.x);
        }

        grid
    }
}

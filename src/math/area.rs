use glam::I16Vec2;
use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub struct Area {
    lines: Vec<Line>,
    min_y: i16,
    max_y: i16,
}

impl Area {
    #[must_use]
    pub fn new(lines: Vec<Line>, min_y: i16) -> Self {
        assert!(!lines.is_empty(), "area must contain at least one line");

        let max_y = min_y
            .checked_add(lines.len() as i16 - 1)
            .unwrap_or_else(|| {
                panic!(
                    "Invalid area parameters: y-coordinate overflow. \
                    Likely due to a large 'min_y' ({}) or too many lines ({}).",
                    min_y,
                    lines.len()
                )
            });

        Self {
            lines,
            min_y,
            max_y,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (i16, &Line)> + '_ {
        self.lines
            .iter()
            .enumerate()
            .map(|(index, line)| (self.min_y + index as i16, line))
    }

    #[must_use]
    pub fn line(&self, y: i16) -> Option<&Line> {
        if y < self.min_y || y > self.max_y {
            return None;
        }

        Some(&self.lines[(y - self.min_y) as usize])
    }

    #[must_use]
    pub fn intersects(&self, other: &Area) -> bool {
        if self.min_y > other.max_y || other.min_y > self.max_y {
            return false;
        }

        let from_y = self.min_y.max(other.min_y);
        let to_y = self.max_y.min(other.max_y);
        for y in from_y..=to_y {
            if self.line(y).unwrap().intersects(other.line(y).unwrap()) {
                return true;
            }
        }

        false
    }

    #[must_use]
    pub fn coverage(&self) -> usize {
        self.lines.iter().filter_map(|line| line.coverage()).sum()
    }

    #[must_use]
    pub fn min_y(&self) -> i16 {
        self.min_y
    }

    #[must_use]
    pub fn max_y(&self) -> i16 {
        self.max_y
    }

    #[must_use]
    pub fn min_x(&self) -> Option<i16> {
        self.lines.iter().filter_map(|line| line.min_x()).min()
    }

    #[must_use]
    pub fn max_x(&self) -> Option<i16> {
        self.lines.iter().filter_map(|line| line.max_x()).max()
    }

    #[must_use]
    pub fn optimize(&self) -> Option<Area> {
        let start = self.lines.iter().position(|line| !line.is_empty())?;
        let end = self.lines.iter().rposition(|line| !line.is_empty())?;

        let mut lines = Vec::with_capacity((end - start) + 1);
        for i in start..=end {
            lines.push(self.lines[i].clone());
        }

        let min_y = (start.min(i32::MAX as usize) as i32).saturating_add(self.min_y as i32);
        let min_y = i16::try_from(min_y).unwrap_or_else(|_| {
            panic!(
                "bug: area optimization failed: min_y overflow. \
                [start index: {}, original min_y: {}, lines.len(): {}]",
                start,
                self.min_y,
                self.lines.len()
            )
        });

        Some(Area::new(lines, min_y))
    }
}

#[derive(Debug, Clone)]
pub struct Line {
    ranges: Vec<Range>,
}

impl Line {
    #[must_use]
    pub fn new(ranges: Vec<Range>) -> Self {
        for i in 1..ranges.len() {
            let range = ranges[i];
            let previous_range = ranges[i - 1];
            assert!(previous_range.to_inclusive <= range.from);
        }
        Self { ranges }
    }

    pub fn iter_x(&self) -> impl Iterator<Item = i16> + '_ {
        self.ranges
            .iter()
            .flat_map(|&range| range.from..=range.to_inclusive)
    }

    #[must_use]
    pub fn intersects(&self, other: &Line) -> bool {
        fn result(line: &Line, other: &Line) -> Option<bool> {
            let min_x = line.min_x()?;
            let max_x = line.max_x()?;
            let other_min_x = other.min_x()?;
            let other_max_x = other.max_x()?;

            Some(max_x >= other_min_x && other_max_x >= min_x)
        }

        result(self, other).unwrap_or(false)
    }

    #[must_use]
    pub fn coverage(&self) -> Option<usize> {
        self.ranges
            .iter()
            .map(|range| (range.to_inclusive - range.from) as usize + 1)
            .try_fold(0, |acc, x| Some(acc + x))
    }

    #[must_use]
    pub fn min_x(&self) -> Option<i16> {
        self.ranges.first().map(|range| range.from())
    }

    #[must_use]
    pub fn max_x(&self) -> Option<i16> {
        self.ranges.last().map(|range| range.to_inclusive())
    }
}

impl Deref for Line {
    type Target = [Range];

    fn deref(&self) -> &Self::Target {
        &self.ranges
    }
}

impl DerefMut for Line {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ranges
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Range {
    from: i16,
    to_inclusive: i16,
}

impl Range {
    #[must_use]
    pub fn new(from: i16, to_inclusive: i16) -> Self {
        assert!(from <= to_inclusive);
        Self { from, to_inclusive }
    }

    #[must_use]
    pub fn from(self) -> i16 {
        self.from
    }

    #[must_use]
    pub fn to_inclusive(self) -> i16 {
        self.to_inclusive
    }
}

/*
 * Methods
 */

#[must_use]
#[allow(clippy::needless_range_loop)]
pub fn find_inner_area(curve: &[I16Vec2]) -> Option<Area> {
    if curve.len() <= 1 {
        return None;
    }

    let (min_y, y_length) = {
        let min_y = curve.iter().min_by_key(|point| point.y)?.y;
        let max_y = curve.iter().max_by_key(|point| point.y)?.y;

        (min_y, (max_y - min_y) as usize + 1)
    };

    let mut anchor_lines: Vec<Vec<i16>> = Vec::with_capacity(y_length);
    anchor_lines.resize_with(y_length, Vec::new);

    {
        let mut last_y_diff = 0;

        for i in 1..curve.len() {
            let point = curve[i];
            let previous_point = curve[i - 1];
            let y_diff = point.y - previous_point.y;

            if y_diff == 0 {
                continue;
            }

            if last_y_diff != y_diff {
                let index = (previous_point.y - min_y) as usize;
                if let Some(&last_anchor) = anchor_lines[index].last() {
                    anchor_lines[index].push(last_anchor);
                }

                last_y_diff = y_diff;
            }

            anchor_lines[(point.y - min_y) as usize].push(point.x);
        }
    }

    {
        let origin = curve[0];
        if let Some(anchors) = anchor_lines.get_mut((origin.y - min_y) as usize) {
            if anchors.len() % 2 != 0 {
                anchors.push(origin.x);
            }
        }
    }

    for anchors in &mut anchor_lines {
        anchors.sort_unstable();
    }

    let mut area: Vec<Vec<Range>> = Vec::with_capacity(y_length);
    area.resize_with(area.capacity(), Vec::new);

    let curve = {
        let mut lines: Vec<Vec<i16>> = Vec::with_capacity(y_length);
        lines.resize_with(y_length, Vec::new);

        for point in curve {
            lines[(point.y - min_y) as usize].push(point.x);
        }
        for line in &mut lines {
            line.sort_unstable();
        }

        lines
    };
    let mut push_range = |x1: i16, x2: i16, y: i16, curve_x_offset: usize| -> Option<usize> {
        debug_assert!(x1 <= x2);
        if x2 - x1 <= 1 {
            return None;
        }

        let line = &curve[(y - min_y) as usize];
        let mut x1_index = line
            .iter()
            .skip(curve_x_offset)
            .position(|&x| x == x1)
            .map(|index| index + curve_x_offset)?;
        let x2_index = line
            .iter()
            .skip(x1_index)
            .position(|&x| x == x2)
            .map(|index| index + x1_index)?;

        let mut actual_x1 = x1;
        let mut actual_x2 = x2;

        for i in (x1_index + 1)..x2_index {
            let x = line[i];
            debug_assert!(x >= actual_x1);

            if x - actual_x1 > 1 {
                break;
            }

            actual_x1 = x;
            x1_index += 1;
        }

        if x2_index != 0 {
            let mut i = x2_index - 1;
            while i > x1_index {
                let x = line[i];
                debug_assert!(actual_x2 >= x);

                if actual_x2 - x > 1 {
                    break;
                }

                actual_x2 = x;
                i -= 1;
            }
        }

        debug_assert!(actual_x1 <= actual_x2);
        if actual_x2 - actual_x1 > 1 {
            area[(y - min_y) as usize].push(Range::new(actual_x1 + 1, actual_x2 - 1));
        }

        Some(x2_index)
    };

    for (y, anchors) in anchor_lines
        .iter()
        .enumerate()
        .map(|(i, line)| (min_y + (i as i16), line))
    {
        if anchors.len() <= 1 {
            continue;
        }

        let mut anchor_index = 0;
        let mut curve_x_offset = 0;

        while anchor_index < anchors.len() - 1 {
            let last_x = push_range(
                anchors[anchor_index],
                anchors[anchor_index + 1],
                y,
                curve_x_offset,
            );

            curve_x_offset = last_x.unwrap_or(curve_x_offset);
            anchor_index += 2;
        }

        if anchors.len() % 2 != 0 {
            push_range(anchors[anchors.len() - 2], anchors[anchors.len() - 1], y, 0);
        }
    }

    Area::new(area.into_iter().map(Line::new).collect(), min_y).optimize()
}

#[must_use]
pub fn cull(back_area: &Area, front_area: &Area) -> Option<Area> {
    let min_y = back_area.min_y().min(front_area.min_y());
    let max_y = back_area.max_y().max(front_area.max_y());

    let mut visible_lines: Vec<Line> = Vec::with_capacity((max_y - min_y) as usize + 1);
    visible_lines.resize_with(visible_lines.capacity(), || Line::new(vec![]));

    let mut back_y = back_area.min_y();
    let mut front_y = front_area.min_y().max(back_y);

    let mut set_visible = |y: i16, line: Line| {
        visible_lines[(y - min_y) as usize] = line;
    };
    macro_rules! next {
        ($($var:ident),+) => {
            $(
                $var += 1;
            )+
            continue;
        };
    }

    while back_y <= back_area.max_y() && front_y <= front_area.max_y() {
        let back_line = back_area.line(back_y).unwrap();
        let front_line = front_area.line(front_y).unwrap();

        match back_y.cmp(&front_y) {
            Ordering::Equal => {
                if !back_line.intersects(front_line) {
                    set_visible(back_y, back_line.clone());
                    next!(back_y);
                }

                let mut back_line_index = 0;
                let mut front_line_index = 0;
                let mut ranges = Vec::new();

                while back_line_index < back_line.len() && front_line_index < front_line.len() {
                    // '()' is the back area range, '[]' is the front area range
                    // '(' or '[' indicates the 'from' point.
                    // ')' or ']' indicates the 'to' point (inclusive).
                    // '...' means 0 or more elements
                    // '___' means 1 or more elements
                    let back = back_line[back_line_index];
                    let front = front_line[front_line_index];

                    if back.to_inclusive() < front.from() {
                        // (...)___[...]
                        ranges.push(back);
                        next!(back_line_index);
                    }
                    if front.to_inclusive() < back.from() {
                        // [...]___(...)
                        next!(front_line_index);
                    }

                    {
                        if back.from() < front.from() {
                            // (___[...?
                            ranges.push(Range::new(back.from(), front.from() - 1));
                        }

                        if front.to_inclusive() < back.to_inclusive() {
                            // ?...]___)
                            ranges.push(Range::new(front.to_inclusive() + 1, back.to_inclusive()));
                            next!(back_line_index, front_line_index);
                        } else {
                            // ?...)...]
                            next!(back_line_index);
                        }
                    }
                }

                back_line
                    .iter()
                    .skip(back_line_index)
                    .for_each(|&range| ranges.push(range));
                set_visible(back_y, Line::new(ranges));
                next!(back_y, front_y);
            }
            Ordering::Greater => {
                next!(front_y);
            }
            Ordering::Less => {
                set_visible(back_y, back_line.clone());
                next!(back_y);
            }
        }
    }

    for y in back_y..=back_area.max_y() {
        set_visible(y, back_area.line(y).unwrap().clone())
    }

    Area::new(visible_lines, min_y).optimize()
}

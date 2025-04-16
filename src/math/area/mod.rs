use crate::math::area::line::{Line, LineError};
use crate::math::area::range::Range;
use crate::math::curve::ClosedCurve;
use crate::math::definition::{Int, IntPoint};
use crate::math::grid::Grid2;
use anyhow::{bail, Result};
use glam::USizeVec2;
use ndarray::Array2;
use std::any::type_name;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt::Debug;

pub mod line;
pub mod range;

#[derive(Debug, Clone)]
pub struct Area {
    grid: Grid2<Option<Line>>,
    min_x: Int,
    max_x: Int,
    coverage: usize,
}

impl Area {
    pub fn line(&self, y: Int) -> Option<&Line> {
        self.grid.line(y)?.as_ref()
    }


    pub fn enumerate(&self) -> impl Iterator<Item = (Int, &Line)> + '_ {
        self.grid
            .enumerate()
            .filter_map(|(y, line)| Some((y, line.as_ref()?)))
    }

    pub fn flatten(&self) -> impl Iterator<Item = IntPoint> + use<'_> {
        self.enumerate()
            .flat_map(|(y, line)| line.flatten().map(move |x| IntPoint::new(x, y)))
    }

    pub fn into_enumerate(self) -> impl Iterator<Item = (Int, Line)> {
        self.grid
            .into_enumerate()
            .filter_map(|(y, line)| Some((y, line?)))
    }


    pub fn min_y(&self) -> Int {
        self.grid.min_y()
    }

    pub fn max_y(&self) -> Int {
        self.grid.max_y()
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


    // TODO Docs
    //  Faster than `intersects`.
    //  May produce false-positives, but never false-negatives.
    pub fn dirty_intersects(&self, other: &Area) -> bool {
        self.intersects_template(other, false)
    }

    pub fn intersects(&self, other: &Area) -> bool {
        self.intersects_template(other, true)
    }


    pub fn split(&self) -> Vec<Area> {
        // Determinism:
        // This set is used only for contains/insert operations.
        let mut visited = HashSet::new();
        let mut areas = Vec::new();

        for (y, line) in self.enumerate() {
            for &range in line.ranges() {
                if let Some(area) = self.flood_fill((y, range), &mut visited) {
                    areas.push(area);
                }
            }
        }

        areas
    }

    pub fn area_behind(&self, front_area: &Area) -> Option<Area> {
        let back_area = self;
        let mut visible_area = Grid2::new(back_area.min_y(), back_area.max_y(), || None);

        let mut back_y = self.min_y();
        let mut front_y = front_area.min_y().max(back_y);

        macro_rules! next {
            ($($var:ident),+) => {
                $(
                    $var += 1;
                )+
                continue;
            };
        }


        while back_y <= back_area.max_y() && front_y <= front_area.max_y() {
            let back_line = back_area.line(back_y);
            let front_line = front_area.line(front_y);

            match back_y.cmp(&front_y) {
                Ordering::Equal => {
                    let Some(back_line) = back_line else {
                        next!(back_y, front_y);
                    };
                    let Some(front_line) = front_line else {
                        visible_area[back_y] = Some(back_line.clone());
                        next!(back_y, front_y);
                    };

                    if !back_line.dirty_intersects(front_line) {
                        visible_area[back_y] = Some(back_line.clone());
                        next!(back_y, front_y);
                    }


                    let mut back_line_index = 0;
                    let mut front_line_index = 0;
                    let mut ranges = Vec::new();

                    while back_line_index < back_line.len() && front_line_index < front_line.len() {
                        // '()' is the back area range, '[]' is the front area range
                        // '(' or '[' represents the 'from' point.
                        // ')' or ']' represents the 'to' point.
                        // '...' means 0 or more elements
                        // '___' means 1 or more elements

                        let back = back_line[back_line_index];
                        let front = front_line[front_line_index];

                        if back.to() < front.from() {
                            // (...)___[...]
                            ranges.push(back);
                            next!(back_line_index);
                        }
                        if front.to() < back.from() {
                            // [...]___(...)
                            next!(front_line_index);
                        }


                        if back.from() < front.from() {
                            if let Some(past_front) = front_line_index
                                .checked_sub(1)
                                .map(|index| front_line[index])
                                .filter(|range| range.to() >= back.from())
                            {
                                // (...[...]___[...?
                                // [...(...]___[...?

                                // Confidence:
                                // Due to the Line guarantees,
                                // `front.from() - past_front.to()` is always > 1.
                                //
                                // Therefore, Range::new should not panic.
                                debug_assert!(front.from() - past_front.to() > 1);
                                ranges.push(Range::new(past_front.to() + 1, front.from() - 1));
                            } else {
                                // (___[...?
                                ranges.push(Range::new(back.from(), front.from() - 1));
                            }
                        }

                        if front.to() < back.to() {
                            if front_line_index
                                .checked_add(1)
                                .filter(|&index| index < front_line.len())
                                .map(|index| front_line[index])
                                .filter(|range| range.from() <= back.to())
                                .is_none()
                            {
                                // ?...]___)
                                ranges.push(Range::new(front.to() + 1, back.to()));

                                // Increment both: back_line_index and front_line_index,
                                // so, `back` range won't be pushed twice
                                // because of the `(...)___[...]` condition.
                                next!(back_line_index, front_line_index);
                            } else {
                                // ?...]___[...]...)
                                // ?...]___[...)...]

                                // This case is processed in the next `(...[...]___[...?` iteration.
                                next!(front_line_index);
                            }
                        } else {
                            // ?...)...]
                            next!(back_line_index);
                        }
                    }

                    back_line
                        .iter()
                        .skip(back_line_index)
                        .for_each(|&range| ranges.push(range));

                    // Confidence:
                    // On paper, `back_line_index` and `front_line_index` are constantly moving forward,
                    // so `ranges` should not overlap.
                    //
                    // TODO Tests
                    //  However, tests wouldn't hurt here...
                    visible_area[back_y] = confident_ranges_to_line(ranges);
                    next!(back_y, front_y);
                }
                Ordering::Greater => {
                    next!(front_y);
                }
                Ordering::Less => {
                    visible_area[back_y] = back_line.cloned();
                    next!(back_y);
                }
            }
        }


        for y in back_y..=back_area.max_y() {
            visible_area[y] = back_area.line(y).cloned();
        }

        Area::try_from(visible_area).ok()
    }


    // TODO Docs
    //  (Int, Range): y, range.
    //  `visited` set is used only for contains/insert operations.
    //
    // There is no check that `self` contains the `start`.
    //
    // Four-way implementation.
    fn flood_fill(&self, start: (Int, Range), visited: &mut HashSet<(Int, Range)>) -> Option<Area> {
        if !visited.insert(start) {
            return None;
        }

        let related_ranges = {
            let mut related_ranges = Vec::new();
            let mut related_ranges_mark = 0;
            related_ranges.push(start);

            while related_ranges_mark < related_ranges.len() {
                let (y, range) = related_ranges[related_ranges_mark];
                let mut search = |y: Int| {
                    if let Some(line) = self.line(y) {
                        if let Some(ranges) = line.query_slice(range) {
                            for &range in ranges {
                                if visited.insert((y, range)) {
                                    related_ranges.push((y, range));
                                }
                            }
                        }
                    }
                };

                search(y + 1);
                search(y - 1);
                related_ranges_mark += 1;

                // Due to the Line guarantees,
                // searching for `x` neighbors is unnecessary.
                // The resulting ranges will not have `x` neighbors.

                // Visualization (example):
                // Where '_' (or ___ if the range is 3 units long) represents a new range,
                //       '.' (or ... if the range is 3 units long) represents an old range.
                //
                //      _____
                //
                //        ↓
                //
                //    ___ _ __
                //      .....
                //         ______
                //
                //        ↓
                //
                //  ____  _ _____
                //    ... . ..
                //   __  ..... ____
                //          ......
                //        ___ _ __
                //
            }

            related_ranges.sort_unstable_by(
                |&(first_y, first_range), &(second_y, second_range)| {
                    first_y.cmp(&second_y).then_with(|| {
                        debug_assert!(!first_range.intersects(second_range));
                        debug_assert_eq!(
                            first_range.from().cmp(&second_range.from()),
                            first_range.to().cmp(&second_range.to())
                        );
                        first_range.from().cmp(&second_range.from())
                    })
                },
            );
            related_ranges
        };

        let grid = {
            let mut grid = Grid2::new(
                related_ranges
                    .first()
                    .expect("related_ranges cannot be empty")
                    .0,
                related_ranges
                    .last()
                    .expect("related_ranges cannot be empty")
                    .0,
                Vec::new,
            );

            for (y, range) in related_ranges {
                grid.push(y, range);
            }

            grid
        };

        // Confidence:
        // This implementation doesn't cut/modify ranges and only uses existing ones.
        // At the end, `related_ranges` are sorted to ensure their proper order.
        //
        // So, as long as `self.lines` don't break their guarantees,
        // their guarantees are preserved here.
        Area::try_from(confident_ranges_to_lines_grid(grid)).ok()
    }

    fn intersects_template(&self, other: &Area, precise: bool) -> bool {
        if self.min_y() > other.max_y() || other.min_y() > self.max_y() {
            return false;
        }
        if self.min_x() > other.max_x() || other.min_x() > self.max_x() {
            return false;
        }

        let from_y = self.min_y().max(other.min_y());
        let to_y = self.max_y().min(other.max_y());

        for y in from_y..=to_y {
            let (self_line, other_line) = {
                match self.line(y).zip(other.line(y)) {
                    None => continue,
                    Some(lines) => lines,
                }
            };

            #[allow(clippy::collapsible_else_if)]
            if precise {
                if self_line.intersects(other_line) {
                    return true;
                }
            } else {
                if self_line.dirty_intersects(other_line) {
                    return true;
                }
            };
        }

        false
    }

    fn shrink_to_fit(&mut self) {
        // TODO Refactor
        //  self
        //   .grid
        //   .enumerate()
        //   .rev() <-- make this to compile?
        //   .find(|(_, line)| line.is_some())

        let mut start = self.min_y();
        let mut end = self.max_y();

        for y in start..=end {
            if self.grid[y].is_some() {
                start = y;
                break;
            }
        }
        for y in (start..=end).rev() {
            if self.grid[y].is_some() {
                end = y;
                break;
            }
        }

        self.grid.shrink_to(start..=end);
    }
}


impl TryFrom<Grid2<Option<Line>>> for Area {
    type Error = ();

    fn try_from(grid: Grid2<Option<Line>>) -> Result<Self, Self::Error> {
        let min_x = grid
            .iter()
            .filter_map(|line| line.as_ref())
            .map(|line| line.min_x())
            .min()
            .ok_or(())?;
        let max_x = grid
            .iter()
            .filter_map(|line| line.as_ref())
            .map(|line| line.max_x())
            .max()
            .ok_or(())?;
        let coverage = grid
            .iter()
            .filter_map(|line| line.as_ref())
            .map(|line| line.coverage())
            .sum();

        let mut area = Self {
            grid,
            min_x,
            max_x,
            coverage,
        };

        area.shrink_to_fit();
        Ok(area)
    }
}


impl PartialEq for Area {
    fn eq(&self, other: &Self) -> bool {
        self.coverage == other.coverage
            && self.min_x == other.min_x
            && self.max_x == other.max_x
            && self.grid == other.grid
    }
}

impl Eq for Area {}


pub fn merge(mut areas: Vec<Area>) -> Option<Area> {
    if areas.is_empty() {
        return None;
    }
    if areas.len() == 1 {
        return areas.pop();
    }

    let mut total_grid = Grid2::new(
        areas.iter().map(|area| area.min_y()).min()?,
        areas.iter().map(|area| area.max_y()).max()?,
        Vec::new,
    );

    for area in areas {
        for (y, mut ranges) in area
            .into_enumerate()
            .map(|(y, line)| (y, line.into_ranges()))
        {
            total_grid.append(y, &mut ranges);
        }
    }


    total_grid.sort_unstable_by_key(|range| range.from());
    let mut merged_grid = Grid2::new(total_grid.min_y(), total_grid.max_y(), Vec::new);

    for (y, total_ranges) in total_grid.into_enumerate() {
        let merged_ranges = &mut merged_grid[y];

        if let Some(&range) = total_ranges.first() {
            merged_ranges.push(range);
        } else {
            continue;
        }

        for total_range in total_ranges.into_iter().skip(1) {
            let last_merged_range = *merged_ranges.last().expect("merged_ranges cannot be empty");
            debug_assert!(last_merged_range.from() <= total_range.from());

            if total_range.intersects(last_merged_range) {
                let range = Range::new(
                    last_merged_range.from(),
                    last_merged_range.to().max(total_range.to()),
                );
                *merged_ranges
                    .last_mut()
                    .expect("merged_ranges cannot be empty") = range;
            } else {
                merged_ranges.push(total_range);
            }
        }
    }

    // Confidence:
    // - `merged_ranges` is sorted.
    // - `merged_ranges` will never overlap, as overlapping ranges have been merged.
    Area::try_from(confident_ranges_to_lines_grid(merged_grid)).ok()
}


// TODO: Performance?
//
// My last attempt at writing an algorithm
// for finding all inner areas within a large, interpolated,
// self-enclosing, and heavily self-intersecting curve
// settled on a basic flood fill algorithm.
//
// Advantages:
//  - Flood fill is simple, or at least simpler than the approaches I tried previously.
//  - Flood fill is accurate because it operates pixel-by-pixel on the interpolated curve.
//
// Why it might be not the best:
//  - Performance: The larger the curve,
//    the more memory must be allocated for the Flood Fill canvas,
//    and the more iterations are required.
//
//
// For curious readers and my future self (with fresh ideas), here's what I tried previously:
//
//  1. Concave Hull (with several implementation variations):
//   Basic point-by-point comparison:
//   | - Take a point.
//   | - Find its squared distance to a central point.
//   | - Compare it with another point.
//   | - Move in the farthest direction.
//   |
//   | Didn't work well; inaccurate in most cases.
//   | Tried "tuning" with angles (prioritizing clockwise direction),
//   | but it didn't significantly improve results.
//
//   Concave Hull on steroids:
//   | - Find all curve self-intersections (relatively easy since the curve is interpolated).
//   |
//   | -- This yields precise intersections (two or more points share the same integer coordinates)
//   | -- or "square" intersections (where paths cross diagonally within adjacent pixels), like:
//   | --  ⬊             ⬈
//   | --    ⬊         ⬈
//   | --      ⬊     ⬈
//   | --        ⬊ ⬈
//   | --        ⬈ ⬊  (2x2 square!)
//   | --      ⬈     ⬊
//   | --    ⬈         ⬊
//   | --  ⬈             ⬊
//   |
//   | - Create a bidirectional graph where intersections are vertices
//   |   and segments between them (composed of original curve points) are edges.
//   | - Compare outgoing edge directions from each vertex
//   |   to choose the "farthest" or most "outward" one (using angle calculations):
//   |
//   | -- fn clockwise_angle(self) -> Scalar {
//   | --   Some(self)
//   | --     .filter(|&it| it != Vec2::ZERO)
//   | --     .map(|it| it.to_angle())
//   | --     .map(|it| (Scalar::PI() / 2.0) - it)
//   | --     .map(|it| it.rem_euclid(2.0 * Scalar::PI()))
//   | --     .unwrap_or(0.0)
//   | -- }
//   | --
//   | -- fn clockwise_direction_angle(origin: Point, from: Point, to: Point) -> Scalar {
//   | --   debug_assert_finite!(origin, from, to);
//   | --
//   | --   let from = from - origin;
//   | --   let to = to - origin;
//   | --   let direction = to - from;
//   | --
//   | --   let from_angle = from.clockwise_angle();
//   | --   let direction_angle = direction.clockwise_angle();
//   | --   debug_eval_finite!((direction_angle - from_angle).rem_euclid(2.0 * PI))
//   | -- }
//   | --
//   | -- Where `from` represents a vertex, and `to` represents a point on an edge.
//   | -- The smaller `clockwise_direction_angle`, the higher the priority.
//   |
//   | - Perform two concave hull runs: one prioritizing clockwise turns, the other counter-clockwise.
//   | This worked much better than the basic version, often achieving 100% accuracy,
//   | but inaccuracies remained, especially with highly "spiky" or complex curves.
//   |
//   | A downside of the strict angular approach is that
//   | if the optimal path locally turns slightly "backwards" relative to the overall hull direction,
//   | it might be ignored.
//   | Fixing this introduces other complexities and potential pitfalls (trust me...).
//   |
//   | A benefit of this angle-based approach is that it tends to trace the full boundary
//   | rather than getting stuck in small local loops,
//   | encouraging completion of the entire hull before returning to the starting vertex.
//
//  Why attempt concave hull? The goal was to find the overall boundary polygon of the curve.
//  This boundary could then potentially be used with an algorithm like sweep-line to identify the inner areas,
//  avoiding the need for a pixel canvas (which seemed potentially faster).
//
//  I still believe there might be a way to implement this without Flood Fill,
//  but unfortunately, I don't have the time currently to pursue it further.
//
// 2. Another approach attempted: Dijkstra's Algorithm:
//  | - Find curve self-intersections.
//  | - Create a bidirectional graph (as described above).
//  | - For each intersection vertex, find the shortest loop back to itself,
//  |   starting the search along each neighboring edge (using edge length as cost).
//  |
//  | The expectation was that this would find all closed loops/areas (potentially nested or overlapping - hence "dirty"),
//  | which could then be "cleaned" (e.g., resolve nesting) using Area::area_behind.
//  |
//  | This approach worked, but was highly inaccurate.
//  | The reason is that even finding all shortest loops,
//  | significant gaps between these loops can remain, failing to identify all contained areas.
//  | Simple visualization of a potential gap:
//  |
//  |     ++++++++++++++++++++++++++++++++
//  |     +                              +
//  |+++++...............................+++++
//  |+    .                             .    +
//  |+      .                         .      +
//  |  +      .         GAP         .      +
//  |    +      .                 .      +
//  |      +      .             .      +
//  |        +      .         .      +
//  |          +      .     .      +
//  |            +      . .      +
//  |              +     +     +
//  |                +   +   +
//  |                  +   +
//  |
//  | One could potentially run Dijkstra multiple times from each node, exploring paths beyond the absolute shortest,
//  | but this becomes computationally expensive very quickly and still doesn't guarantee finding all areas accurately.
//
// Perhaps revisiting this problem with more time could yield a non-flood-fill solution,
// but for now, the flood fill approach remains.
pub fn find_inner_areas(curve: &ClosedCurve) -> Result<Vec<Area>> {
    // The curve's first point is always the same as the last point,
    // so we have points 5 here.
    //    .
    //  .   .
    //    .
    if curve.len() < 5 {
        return Ok(vec![]);
    }

    let (min_x, min_y, max_x, max_y) = {
        let mut min_x = Int::MAX;
        let mut min_y = Int::MAX;
        let mut max_x = Int::MIN;
        let mut max_y = Int::MIN;

        for point in curve.iter() {
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }

        // We need to exclude area outside the curve,
        // we need to create a gap for Flood Fill algorithm that will perform that.
        let min_x = min_x.checked_sub(2);
        let min_y = min_y.checked_sub(2);
        let max_x = max_x.checked_add(2);
        let max_y = max_y.checked_add(2);

        if [min_x, min_y, max_x, max_y].iter().any(|it| it.is_none()) {
            bail!(
                "failed to create a gap for the flood fill algorithm: curve is too close to the Int({}) overflow limit",
                type_name::<Int>()
            );
        }

        (
            min_x.unwrap(),
            min_y.unwrap(),
            max_x.unwrap(),
            max_y.unwrap(),
        )
    };

    // Our pixel canvas with boolean flags,
    // where `true` means already visited.
    let mut canvas = {
        let width = (max_x - min_x) as usize;
        let height = (max_y - min_y) as usize;
        Array2::from_elem((height, width), false)
    };


    let index_of = |point: IntPoint| -> [usize; 2] {
        let x = (point.x - min_x) as usize;
        let y = (point.y - min_y) as usize;
        [y, x]
    };

    let position_of_x = |pos: usize| -> Int { min_x + (pos as Int) };
    let position_of_y = |pos: usize| -> Int { min_y + (pos as Int) };

    // Mark curve points as boundaries,
    // then exclude the outer area that is not part of the curve.
    for &point in curve.iter() {
        canvas[index_of(point)] = true;
    }
    flood_fill(
        &mut canvas,
        USizeVec2::ZERO,
        false,
        true,
        position_of_x,
        position_of_y,
    );


    // Finally, find our inner areas.
    let mut areas = Vec::new();
    for y in 0..canvas.nrows() {
        for x in 0..canvas.ncols() {
            if let Some(area) = flood_fill(
                &mut canvas,
                USizeVec2::new(x, y),
                false,
                true,
                position_of_x,
                position_of_y,
            ) {
                areas.push(area);
            }
        }
    }

    Ok(areas)
}

fn flood_fill<T, XC, YC>(
    canvas: &mut Array2<T>,
    start: USizeVec2,
    target_value: T,
    fill_value: T,
    x_converter: XC,
    y_converter: YC,
) -> Option<Area>
where
    T: Copy + Eq + Debug,
    XC: Fn(usize) -> Int,
    YC: Fn(usize) -> Int,
{
    if canvas[[start.y, start.x]] != target_value {
        return None;
    }

    let mut points_to_discover = Vec::new();
    let mut ranges = Vec::new();
    points_to_discover.push(start);

    while let Some(point) = points_to_discover.pop() {
        let x_range = {
            let mut row = canvas.row_mut(point.y);
            if row[point.x] != target_value {
                continue;
            }

            let mut fill = |direction: isize| -> usize {
                debug_assert_eq!(direction.abs(), 1);

                let mut x = point.x;
                let bound = row.len();

                loop {
                    row[x] = fill_value;
                    if (direction == -1 && x == 0) || (direction == 1 && x >= (bound - 1)) {
                        return x;
                    }

                    let past_x = x;
                    x = x
                        .checked_add_signed(direction)
                        .expect("must never overflow");

                    if row[x] != target_value {
                        return past_x;
                    }
                }
            };

            let from = fill(-1);
            let to = fill(1);
            from..=to
        };


        let mut extend = |y: usize| {
            let row = canvas.row(y);
            let row_length = row.len();

            if *x_range.start() >= row_length {
                return;
            }

            let x_range = *x_range.start()..=(*x_range.end()).min(row_length - 1);
            let mut past_x: Option<usize> = None;

            for x in x_range {
                if row[x] == target_value {
                    if let Some(past_x) = past_x {
                        debug_assert!(x >= past_x);

                        if (x - past_x) > 1 {
                            points_to_discover.push(USizeVec2::new(x, y));
                        }
                    } else {
                        points_to_discover.push(USizeVec2::new(x, y));
                    }

                    past_x = Some(x);
                }
            }
        };

        if point.y != 0 {
            extend(point.y - 1);
        }
        if point.y < canvas.nrows() - 1 {
            extend(point.y + 1);
        }

        ranges.push((point.y, x_range));
    }

    if ranges.is_empty() {
        return None;
    }


    let ranges = ranges
        .into_iter()
        .map(|(y, range)| {
            let y = y_converter(y);
            let x_from = x_converter(*range.start());
            let x_to = x_converter(*range.end());

            (y, Range::new(x_from.min(x_to), x_from.max(x_to)))
        })
        .collect::<Vec<_>>();

    let mut grid = {
        let min_y = ranges
            .iter()
            .min_by_key(|(y, _)| y)
            .expect("ranges cannot be empty")
            .0;
        let max_y = ranges
            .iter()
            .max_by_key(|(y, _)| y)
            .expect("ranges cannot be empty")
            .0;
        Grid2::new(min_y, max_y, Vec::new)
    };

    for (y, range) in ranges {
        grid.push(y, range);
    }
    grid.sort_unstable_by_key(|range| range.from());


    // Confidence:
    // - `ranges` cannot overlap because we only visit pixels once.
    // - `grid` is sorted.
    Area::try_from(confident_ranges_to_lines_grid(grid)).ok()
}


fn confident_ranges_to_line(ranges: Vec<Range>) -> Option<Line> {
    match Line::try_from(ranges) {
        Ok(line) => Some(line),
        Err(LineError::Empty) => None,
        Err(LineError::Invalid) => {
            panic!("failed to transform Vec<Range> into Line: invalid ranges")
        }
    }
}

fn confident_ranges_to_lines_grid(grid: Grid2<Vec<Range>>) -> Grid2<Option<Line>> {
    grid.transform(confident_ranges_to_line)
}

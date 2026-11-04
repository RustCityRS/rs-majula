use rs_grid::CoordGrid;
use std::ops::{Index, IndexMut};

pub struct Waypoints {
    pub index: i32,
    first: CoordGrid,
    rest: Option<Box<[CoordGrid; 24]>>,
}

impl Waypoints {
    pub const CAPACITY: usize = 25;
    const ZERO: CoordGrid = CoordGrid::from(0);
    pub const fn new() -> Self {
        Self {
            first: CoordGrid::from(0),
            index: -1,
            rest: None,
        }
    }
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&CoordGrid> {
        match index {
            0 => Some(&self.first),
            1..=24 => Some(match &self.rest {
                Some(rest) => &rest[index - 1],
                None => &Self::ZERO,
            }),
            _ => None,
        }
    }
    pub fn snapshot(&self) -> [CoordGrid; 25] {
        let mut result = [CoordGrid::from(0); 25];
        result[0] = self.first;
        if let Some(rest) = &self.rest {
            result[1..].copy_from_slice(&**rest);
        }
        result
    }

    pub fn restore(&mut self, value: [CoordGrid; 25]) {
        self.first = value[0];
        if self.rest.is_some() || value[1..].iter().any(|v| v.packed() != 0) {
            let rest = self
                .rest
                .get_or_insert_with(|| Box::new([CoordGrid::from(0); 24]));
            rest.copy_from_slice(&value[1..]);
        }
    }
}

impl Default for Waypoints {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<usize> for Waypoints {
    type Output = CoordGrid;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap_or_else(|| {
            panic!("index out of bounds: the len is 25 but the index is {index}")
        })
    }
}

impl IndexMut<usize> for Waypoints {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.first,
            1..=24 => &mut self
                .rest
                .get_or_insert_with(|| Box::new([CoordGrid::from(0); 24]))[index - 1],
            _ => panic!("index out of bounds: the len is 25 but the index is {index}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Waypoints;
    use rs_grid::CoordGrid;

    #[test]
    fn unallocated_slots_read_as_zero() {
        let waypoints = Waypoints::new();
        assert_eq!(waypoints.index, -1);
        for index in 0..Waypoints::CAPACITY {
            assert_eq!(waypoints[index].packed(), 0);
        }
        assert!(waypoints.get(Waypoints::CAPACITY).is_none());
        assert!(waypoints.rest.is_none());
    }

    #[test]
    fn replacing_first_waypoint_preserves_other_slots() {
        let mut waypoints = Waypoints::new();
        let original = [CoordGrid::from(42); Waypoints::CAPACITY];
        waypoints.restore(original);
        waypoints[0] = CoordGrid::from(7);
        let mut expected = original;
        expected[0] = CoordGrid::from(7);
        assert_eq!(waypoints.snapshot(), expected);
        waypoints.restore(original);
        assert_eq!(waypoints.snapshot(), original);
    }

    #[test]
    fn restoring_zero_snapshot_clears_allocated_slots() {
        let mut waypoints = Waypoints::new();
        waypoints[24] = CoordGrid::from(123);
        let zero = Waypoints::new().snapshot();
        waypoints.restore(zero);
        assert_eq!(waypoints.snapshot(), zero);
    }

    #[test]
    #[should_panic(expected = "index out of bounds: the len is 25 but the index is 25")]
    fn indexing_past_the_buffer_panics() {
        let _ = Waypoints::new()[25];
    }
}

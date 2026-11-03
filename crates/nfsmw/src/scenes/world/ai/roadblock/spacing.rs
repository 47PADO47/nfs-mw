//! Spacing a roadblock's cars by their real size. [ours]
//!
//! The layouts' car offsets suit the patrol cars: the slots can stand about 4.3 m apart along a line of cars nose to
//! tail. A heavy roadblock's SUVs, or a big cop model taking a slot, are longer than that, and two cars created inside
//! each other were thrown onto each other by the contact solver (one car ended up half on the other's roof). Before
//! the cars are created each one is moved, in the order of the layout, just clear of the cars before it, along the
//! shortest way out of the overlap and only in the road's plane. Slots that do not overlap are left exactly where the
//! layout puts them. The road-edge check runs on the moved slots.

use glam::Vec2;

/// The gap left between two cars that had to be moved apart, metres.
const GAP: f32 = 0.3;
/// The farthest one car is moved; a car that would have to go farther keeps the spot it reached.
const MAX_SHIFT: f32 = 4.0;
/// Rounds of the pass over the earlier cars, as a move away from one car can touch another.
const ROUNDS: usize = 4;

/// A car's footprint in the road plane: its centre, the unit direction it faces, and its half length and half width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Footprint {
    pub centre: Vec2,
    pub facing: Vec2,
    pub half_length: f32,
    pub half_width: f32,
}

impl Footprint {
    fn axes(&self) -> [Vec2; 2] {
        [self.facing, self.facing.perp()]
    }

    /// Half the footprint's extent along `axis` (a unit vector).
    fn reach(&self, axis: Vec2) -> f32 {
        self.facing.dot(axis).abs() * self.half_length + self.facing.perp().dot(axis).abs() * self.half_width
    }
}

/// How far `a` must move, and which way, to clear `b` by `GAP`: the shortest of the four box axes. `None` when they
/// are already clear.
fn way_out(a: &Footprint, b: &Footprint) -> Option<Vec2> {
    let between = a.centre - b.centre;
    let mut best: Option<(f32, Vec2)> = None;
    for axis in a.axes().into_iter().chain(b.axes()) {
        let depth = a.reach(axis) + b.reach(axis) + GAP - between.dot(axis).abs();
        if depth <= 0.0 {
            return None;
        }
        if best.is_none_or(|(d, _)| depth < d) {
            let away = match between.dot(axis) < 0.0 {
                true => -axis,
                false => axis,
            };
            best = Some((depth, away));
        }
    }
    best.map(|(depth, away)| away * depth)
}

/// Moves each car clear of the cars before it, in order. Returns the new centres.
pub(super) fn separate(cars: &[Footprint]) -> Vec<Vec2> {
    let mut moved: Vec<Footprint> = Vec::with_capacity(cars.len());
    for car in cars {
        let mut here = *car;
        for _ in 0..ROUNDS {
            let mut clear = true;
            for before in &moved {
                if let Some(push) = way_out(&here, before) {
                    here.centre += push;
                    clear = false;
                }
            }
            if clear {
                break;
            }
        }
        let shift = here.centre - car.centre;
        if shift.length() > MAX_SHIFT {
            here.centre = car.centre + shift.normalize() * MAX_SHIFT;
        }
        moved.push(here);
    }
    moved.into_iter().map(|car| car.centre).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn car(x: f32, y: f32, facing: Vec2, half_length: f32) -> Footprint {
        Footprint { centre: Vec2::new(x, y), facing, half_length, half_width: 1.0 }
    }

    #[test]
    fn cars_that_fit_stay_where_the_layout_puts_them() {
        let cars = [car(0.0, 0.0, Vec2::X, 2.4), car(6.8, 0.0, Vec2::NEG_X, 2.4)];
        assert_eq!(separate(&cars), vec![Vec2::new(0.0, 0.0), Vec2::new(6.8, 0.0)]);
    }

    #[test]
    fn long_cars_nose_to_tail_are_moved_apart_along_their_line() {
        // Two 5.2 m SUVs 4.3 m apart on the same line: 0.9 m of overlap plus the gap.
        let cars = [car(0.0, 0.0, Vec2::X, 2.6), car(4.3, 0.0, Vec2::NEG_X, 2.6)];
        let got = separate(&cars);
        assert_eq!(got[0], Vec2::ZERO, "the first car keeps its slot");
        assert!((got[1].x - (5.2 + GAP)).abs() < 1e-4 && got[1].y.abs() < 1e-4, "{:?}", got[1]);
    }

    #[test]
    fn a_car_beside_another_is_moved_sideways() {
        let cars = [car(0.0, 0.0, Vec2::Y, 2.6), car(1.5, 0.5, Vec2::Y, 2.6)];
        let got = separate(&cars);
        assert!((got[1].x - (2.0 + GAP)).abs() < 1e-4, "out the short way: {:?}", got[1]);
        assert!((got[1].y - 0.5).abs() < 1e-4);
    }

    #[test]
    fn a_turned_car_is_cleared_of_its_neighbour_box_to_box() {
        let diagonal = Vec2::new(1.0, 1.0).normalize();
        let cars = [car(0.0, 0.0, Vec2::X, 2.6), car(3.0, 1.0, diagonal, 2.6)];
        let got = separate(&cars);
        let moved = Footprint { centre: got[1], ..cars[1] };
        assert!(way_out(&moved, &cars[0]).is_none(), "clear after the move: {:?}", got[1]);
    }

    #[test]
    fn a_third_car_clears_both_cars_before_it() {
        let cars = [car(0.0, 0.0, Vec2::X, 2.6), car(4.3, 0.0, Vec2::X, 2.6), car(8.6, 0.0, Vec2::X, 2.6)];
        let got = separate(&cars);
        let placed: Vec<Footprint> = got.iter().zip(&cars).map(|(&c, f)| Footprint { centre: c, ..*f }).collect();
        for i in 0..3 {
            for j in 0..i {
                assert!(way_out(&placed[i], &placed[j]).is_none(), "{i} and {j} overlap: {got:?}");
            }
        }
    }

    #[test]
    fn no_car_moves_more_than_the_limit() {
        let cars = [car(0.0, 0.0, Vec2::X, 2.6), car(0.0, 0.0, Vec2::X, 2.6), car(0.0, 0.0, Vec2::X, 2.6)];
        for (got, from) in separate(&cars).iter().zip(&cars) {
            assert!(got.distance(from.centre) <= MAX_SHIFT + 1e-4);
        }
    }
}

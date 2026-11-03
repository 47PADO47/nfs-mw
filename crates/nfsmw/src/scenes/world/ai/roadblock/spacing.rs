//! Spacing a roadblock's cars by their real size. [ours]
//!
//! The layouts' car offsets suit the patrol cars: the slots can stand about 4.3 m apart along a line of cars nose to
//! tail. A heavy roadblock's SUVs, or a big cop model taking a slot, are longer than that, and two cars created inside
//! each other were thrown onto each other by the contact solver (one car ended up half on the other's roof). Before
//! the cars are created each one is moved, in the order of the layout, just clear of the cars before it, along the
//! shortest way out of the overlap and only in the road's plane. Slots that do not overlap are left exactly where the
//! layout puts them. The road-edge check runs on the moved slots.
//!
//! A car can also stand in a wall: a slot near the road's edge, a narrow street or a car moved by the spacing. Each car
//! looks from its centre to its corners and the middles of its sides; a wall met on the way moves it back out along the
//! wall's normal (`clear_walls`), and the spacing runs again so the moved car does not land on another.

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

/// A wall met by the segment from `from` to `to` (road plane): how far along it as a fraction, and the wall's horizontal
/// normal on the side of `from`.
pub(super) type WallProbe<'a> = dyn Fn(Vec2, Vec2) -> Option<(f32, Vec2)> + 'a;

/// Moves each car out of the walls `probe` finds between its centre and its outline. Returns the new centres.
pub(super) fn clear_walls(cars: &[Footprint], probe: &WallProbe<'_>) -> Vec<Vec2> {
    cars.iter()
        .map(|car| {
            let mut here = *car;
            for _ in 0..ROUNDS {
                let mut push = Vec2::ZERO;
                for point in outline(&here) {
                    let reach = point - here.centre;
                    let Some((t, normal)) = probe(here.centre, point + reach.normalize_or_zero() * GAP) else {
                        continue;
                    };
                    let length = reach.length() + GAP;
                    let depth = length * (1.0 - t.clamp(0.0, 1.0)) * reach.normalize_or_zero().dot(-normal).max(0.0);
                    let along = normal * depth;
                    push = Vec2::new(max_abs(push.x, along.x), max_abs(push.y, along.y));
                }
                if push == Vec2::ZERO {
                    break;
                }
                here.centre += push;
            }
            let shift = here.centre - car.centre;
            match shift.length() > MAX_SHIFT {
                true => car.centre + shift.normalize() * MAX_SHIFT,
                false => here.centre,
            }
        })
        .collect()
}

/// The larger in size of two pushes along one axis.
fn max_abs(a: f32, b: f32) -> f32 {
    match b.abs() > a.abs() {
        true => b,
        false => a,
    }
}

/// The corners of a footprint and the middles of its sides.
fn outline(car: &Footprint) -> [Vec2; 8] {
    let ahead = car.facing * car.half_length;
    let side = car.facing.perp() * car.half_width;
    let c = car.centre;
    [c + ahead + side, c + ahead - side, c - ahead + side, c - ahead - side, c + ahead, c - ahead, c + side, c - side]
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

    /// A straight wall along `x = wall`, facing -x.
    fn wall_at(wall: f32) -> impl Fn(Vec2, Vec2) -> Option<(f32, Vec2)> {
        move |from: Vec2, to: Vec2| {
            (from.x < wall && to.x > wall).then(|| ((wall - from.x) / (to.x - from.x), Vec2::NEG_X))
        }
    }

    #[test]
    fn a_car_in_a_wall_is_moved_out_of_it() {
        // An SUV across the road whose nose reaches 1 m into a wall at x = 3.
        let car = Footprint { centre: Vec2::ZERO, facing: Vec2::X, half_length: 4.0 - 0.0, half_width: 1.0 };
        let got = clear_walls(&[car], &wall_at(3.0));
        assert!((got[0].x - (3.0 - 4.0 - GAP)).abs() < 1e-3, "{:?}", got[0]);
        assert!(got[0].y.abs() < 1e-4);
    }

    #[test]
    fn a_car_clear_of_the_walls_stays() {
        let car = Footprint { centre: Vec2::ZERO, facing: Vec2::X, half_length: 2.6, half_width: 1.0 };
        assert_eq!(clear_walls(&[car], &wall_at(10.0)), vec![Vec2::ZERO]);
    }
}

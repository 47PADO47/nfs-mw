//! The tail pipes of a converted car, from the car's own 2012 data (`docs/formats/chameleon-lamps.md`, "Tail pipes").
//!
//! The 2012 game plays the nitrous flames (and the effects that follow them) at a locator group named by the car's spot
//! effect records: the records whose state bit is 8 or 9 (nitrous pressed, nitrous active). Each locator of the group is
//! one pipe, and its +z axis is the way the flame shoots. A car whose pipes are one side of a symmetrical pair has one
//! locator and the mirror flag of its flame record set. A car with spot effect records but none for the nitrous has no
//! pipe at all (the Tesla, an electric car, and the SWAT van): it gets no exhaust marker, so the game plays no flame and
//! no pop at the tail.

use blackbox_solid::PositionMarker;
use chameleon_vehicle::{Lamps, SpotEffect};

use crate::markers::{self, Spot, distance, marker_at};

/// The state bits of the nitrous flame records: nitrous pressed and nitrous active.
const NITROUS_BITS: [u16; 2] = [8, 9];
/// Two pipes closer than this, metres, are one.
const SAME_PIPE: f32 = 0.02;
/// A pipe this close to the centre plane, metres, is a centre pipe.
const CENTRE_BAND: f32 = 0.02;
/// A group's locator this close to the car's origin (x and z, engine space), metres, is not placed: the 62 cars that share
/// the default group (0, 0.35, 0) put their non-nitrous effects there.
const UNPLACED: f32 = 1e-3;

/// What a car's records say about its pipes.
#[derive(Debug, Clone, PartialEq)]
pub enum Pipes {
    /// The pipes, one marker each.
    Markers(Vec<PositionMarker>),
    /// The car has effect records and none of them is a nitrous flame: it has no tail pipe.
    None,
    /// The car's bundle has no effect records to read.
    Unknown,
}

/// The pipe markers of a car (`to_car` turns a point of the 2012 vehicle space into the game's car space).
pub fn exhaust_markers(lamps: &Lamps, to_car: &dyn Fn([f32; 3]) -> [f32; 3]) -> Pipes {
    if lamps.spot_effects.is_empty() {
        return Pipes::Unknown;
    }
    let spots = pipe_spots(lamps, to_car);
    if spots.is_empty() {
        return Pipes::None;
    }
    Pipes::Markers(spots.into_iter().map(|spot| marker_at(name_of(&spot), spot)).collect())
}

/// The marker name by the side of the car a pipe is on.
fn name_of(spot: &Spot) -> &'static str {
    match spot.at[1] {
        y if y > CENTRE_BAND => "LEFT_EXHAUST",
        y if y < -CENTRE_BAND => "RIGHT_EXHAUST",
        _ => "EXHAUST",
    }
}

fn pipe_spots(lamps: &Lamps, to_car: &dyn Fn([f32; 3]) -> [f32; 3]) -> Vec<Spot> {
    let nitrous: Vec<&SpotEffect> = lamps.spot_effects.iter().filter(|e| NITROUS_BITS.contains(&e.state_bit)).collect();
    let mut groups: Vec<u32> = nitrous.iter().map(|effect| effect.group).collect();
    groups.dedup();
    let mut found: Vec<Spot> = Vec::new();
    for id in groups {
        let Some(group) = lamps.group(id) else { continue };
        // The flame record (bit 9) says whether the group's locators are mirrored; the bit 8 record stands in for a car
        // that has none.
        let flame = |bit| nitrous.iter().find(|e| e.group == id && e.state_bit == bit);
        let mirrored = flame(9).or_else(|| flame(8)).is_some_and(|effect| effect.mirrored);
        for locator in &group.locators {
            let [x, _, z] = locator.position;
            if x.abs() < UNPLACED && z.abs() < UNPLACED {
                continue;
            }
            let at = to_car(locator.position);
            let (turned, origin) = (to_car(locator.forward()), to_car([0.0; 3]));
            let flame_way = unit([turned[0] - origin[0], turned[1] - origin[1], turned[2] - origin[2]]);
            let sides: &[f32] = match mirrored {
                true => &[1.0, -1.0],
                false => &[1.0],
            };
            for &side in sides {
                let spot = Spot {
                    at: [at[0], side * at[1], at[2]],
                    shines: [flame_way[0], side * flame_way[1], flame_way[2]],
                };
                if !found.iter().any(|known| distance(known.at, spot.at) < SAME_PIPE) {
                    found.push(spot);
                }
            }
        }
    }
    found
}

/// A unit vector along `v`; backwards when `v` has no length.
fn unit(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length < 1e-6 {
        return [-1.0, 0.0, 0.0];
    }
    [v[0] / length, v[1] / length, v[2] / length]
}

/// The markers the body carries for the car's pipes: the car's own, or, for a car whose bundle has no effect records, the
/// ones [`markers::tail_pipes`] derives from the body's shape.
pub fn for_body(lamps: &Lamps, to_car: &dyn Fn([f32; 3]) -> [f32; 3], body: &[[f32; 3]]) -> Vec<PositionMarker> {
    match exhaust_markers(lamps, to_car) {
        Pipes::Markers(markers) => markers,
        Pipes::None => Vec::new(),
        Pipes::Unknown => markers::tail_pipes(body),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_hash::bstring_hash;
    use chameleon_vehicle::{Locator, LocatorGroup};

    fn to_car(p: [f32; 3]) -> [f32; 3] {
        [p[2], p[0], p[1]]
    }

    /// Half a turn about x: the locator's +z points back.
    const BACK: [f32; 4] = [1.0, 0.0, 0.0, 0.0];

    fn car(locators: &[[f32; 3]], mirrored: bool, bits: &[u16]) -> Lamps {
        let group = LocatorGroup {
            id: 7,
            locators: locators.iter().map(|&position| Locator { rotation: BACK, position }).collect(),
        };
        let spot_effects = bits.iter().map(|&state_bit| SpotEffect { group: 7, state_bit, mirrored }).collect();
        Lamps { groups: vec![group], spot_effects, ..Default::default() }
    }

    fn markers(pipes: Pipes) -> Vec<PositionMarker> {
        match pipes {
            Pipes::Markers(markers) => markers,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_group_of_both_sides_is_one_marker_per_locator_shooting_backwards() {
        let lamps = car(&[[-0.5, 0.3, -2.1], [-0.4, 0.3, -2.2], [0.4, 0.3, -2.2], [0.5, 0.3, -2.1]], false, &[8, 9]);
        let found = markers(exhaust_markers(&lamps, &to_car));
        assert_eq!(found.len(), 4);
        assert_eq!(found.iter().filter(|m| m.name_hash == bstring_hash("LEFT_EXHAUST")).count(), 2);
        assert_eq!(found.iter().filter(|m| m.name_hash == bstring_hash("RIGHT_EXHAUST")).count(), 2);
        assert!(found.iter().all(|m| m.matrix[8] < -0.99), "the z axis is the way the flame shoots");
        assert_eq!(found[0].translation(), [-2.1, -0.5, 0.3]);
    }

    #[test]
    fn a_mirrored_flame_record_makes_the_pipe_on_the_other_side() {
        let lamps = car(&[[-0.32, 0.96, -1.06]], true, &[8, 9]);
        let found = markers(exhaust_markers(&lamps, &to_car));
        let mut ys: Vec<f32> = found.iter().map(|m| m.translation()[1]).collect();
        ys.sort_by(f32::total_cmp);
        assert_eq!(ys, vec![-0.32, 0.32]);
        // The high pipes of a rear-deck exhaust stay high.
        assert!(found.iter().all(|m| m.translation()[2] == 0.96));
    }

    #[test]
    fn a_single_unmirrored_locator_is_one_pipe() {
        let found = markers(exhaust_markers(&car(&[[-0.8, 0.24, -0.71]], false, &[8, 9]), &to_car));
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn a_pipe_on_the_centre_plane_is_the_centre_exhaust() {
        let found = markers(exhaust_markers(&car(&[[0.0, 0.5, -2.3]], true, &[9]), &to_car));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name_hash, bstring_hash("EXHAUST"));
    }

    #[test]
    fn a_car_with_effect_records_but_no_nitrous_flame_has_no_pipe() {
        // The Tesla: its effects are at the default group, none is a nitrous record.
        let lamps = car(&[[0.0, 0.35, 0.0]], false, &[12, 20]);
        assert_eq!(exhaust_markers(&lamps, &to_car), Pipes::None);
        assert!(for_body(&lamps, &to_car, &[[-2.0, 0.0, 0.5], [2.0, 0.0, 0.5]]).is_empty());
    }

    #[test]
    fn a_nitrous_record_at_the_unplaced_default_group_is_no_pipe() {
        let lamps = car(&[[0.0, 0.35, 0.0]], false, &[8, 9]);
        assert_eq!(exhaust_markers(&lamps, &to_car), Pipes::None);
    }

    #[test]
    fn a_car_with_no_effect_records_falls_back_to_the_pipes_the_body_suggests() {
        let body = [[-2.0, -0.9, 0.1], [-2.0, 0.9, 1.3], [2.0, 0.9, 0.1]];
        let found = for_body(&Lamps::default(), &to_car, &body);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name_hash, bstring_hash("LEFT_EXHAUST"));
    }

    /// Every car of the 2012 install: how many pipes it gets and where (`MW2012_GAME_DIR`, `--nocapture` prints the table).
    /// Every pipe sits on the car's body (the BMW 1M, whose body is the tie-in model's, excepted), and the cars without a nitrous
    /// flame record are the electric Tesla and the SWAT van.
    #[test]
    #[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
    fn every_car_has_pipes_on_its_body_except_the_ones_without_a_flame() {
        use crate::fit::to_car_space;
        use crate::source::Source;
        let Some(root) = std::env::var_os("MW2012_GAME_DIR") else { return };
        let source = Source::open(std::path::Path::new(&root)).unwrap();
        let (mut without, mut unknown) = (Vec::new(), Vec::new());
        for file in source.cars() {
            let car = source.load(file).unwrap();
            let name = source.object_name(file).unwrap_or_else(|| format!("VEH_{}", file.number));
            let body: Vec<[f32; 3]> = car.vehicle.body[0]
                .iter()
                .flat_map(|mesh| mesh.vertices.iter().map(|v| to_car_space(v.position)))
                .collect();
            let (low, high) = body.iter().fold(([f32::MAX; 3], [f32::MIN; 3]), |(lo, hi), p| {
                (std::array::from_fn(|i| lo[i].min(p[i])), std::array::from_fn(|i| hi[i].max(p[i])))
            });
            let found = match exhaust_markers(&car.lamps, &to_car_space) {
                Pipes::Markers(found) => found,
                Pipes::None => {
                    without.push(name.clone());
                    println!("{name} ({}): no pipe", file.number);
                    continue;
                }
                Pipes::Unknown => {
                    unknown.push(name.clone());
                    println!("{name} ({}): no effect records", file.number);
                    continue;
                }
            };
            let list: Vec<String> = found
                .iter()
                .map(|m| {
                    let [x, y, z] = m.translation();
                    format!("({x:.2}, {y:.2}, {z:.2}) -> ({:.2}, {:.2}, {:.2})", m.matrix[8], m.matrix[9], m.matrix[10])
                })
                .collect();
            println!("{name} ({}): {} pipes {}", file.number, found.len(), list.join(" "));
            // The body's own surface around each pipe: its lowest and highest point within 0.25 m (x and y).
            for m in &found {
                let at = m.translation();
                let near: Vec<f32> = body
                    .iter()
                    .filter(|p| (p[0] - at[0]).abs() < 0.25 && (p[1] - at[1]).abs() < 0.25)
                    .map(|p| p[2])
                    .collect();
                let range = near.iter().fold((f32::MAX, f32::MIN), |(lo, hi), z| (lo.min(*z), hi.max(*z)));
                println!(
                    "    pipe z {:.2}: body there {:.2} to {:.2} ({} points)",
                    at[2],
                    range.0,
                    range.1,
                    near.len()
                );
            }
            // The 2012 bundle of the BMW 1M holds the pipes of the car with the body of the Cars tie-in model (Lightning
            // McQueen), 0.3 m beyond its body: its numbers are the car's own and are kept.
            if name.starts_with("BMW_1M") {
                continue;
            }
            for m in &found {
                let at = m.translation();
                let inside = (0..3).all(|i| at[i] > low[i] - 0.15 && at[i] < high[i] + 0.15);
                assert!(inside, "{name}: a pipe at {at:?} is off the body {low:?} {high:?}");
            }
        }
        println!("without a pipe: {without:?}; without effect records: {unknown:?}");
        assert!(without.iter().any(|name| name.starts_with("Tesla")), "{without:?}");
    }
}

//! A bike meets a kerb or a wall in the world, through the path the game uses (`CarSim::step` with a collision world: the wall probes,
//! the body double, what `BikeRide` takes back from the contacts, and the fall tests), not through the library's ground alone
//! (`docs/specs/ride6-feel-round10.md`).

use std::sync::Arc;

use blackbox_collision::{
    Article, BARRIER_TWO_SIDED, Barrier, CollisionPack, CollisionWorld, Instance, PackedVert, STRIP_FACING_UNKNOWN,
    Strip,
};
use blackbox_vehicle::bike::BikeSpec;
use nfsmw_data::car::physics::{CarBounds, CarPhysics, SurfaceTable};

use super::*;
use crate::scenes::world::drive::ground::WorldGround;

const BOUNDS: CarBounds = CarBounds { half_dimensions: Vec3::new(0.42, 0.64, 1.06), pivot: Vec3::new(0.0, 0.64, 0.0) };
/// Where the kerb is along the road (physics z, m): far enough for the bike to settle at 35 m/s first.
const KERB_AT: f32 = 60.0;
/// Half the length of a kerb along its line (m), and how far the road reaches on its low side and on the raised side beyond it.
const SPAN: f32 = 90.0;
const REACH: f32 = 110.0;

/// A strip of `quad` (four corners in strip order, world space) with its centre at the first corner.
fn strip(quad: [[f32; 3]; 4]) -> Strip {
    let centre = quad[0];
    let verts = quad
        .map(|p| PackedVert {
            x: ((p[0] - centre[0]) * 128.0).round() as i16,
            y: ((p[1] - centre[1]) * 128.0).round() as i16,
            z: ((p[2] - centre[2]) * 128.0).round() as i16,
            surface: 0,
            flags: 0,
        })
        .to_vec();
    Strip { center: centre, radius: 300.0, flags: STRIP_FACING_UNKNOWN, verts }
}

/// A kerb: a line across the road that the ground rises at, with a wall of the packs on it if asked.
#[derive(Clone, Copy)]
struct Kerb {
    height: f32,
    /// The rise takes this many metres across the line (0 for a vertical face).
    bevel: f32,
    /// A wall of the packs (a barrier) stands on the line, as an edge of the road is authored.
    wall: bool,
    /// The angle of the line's normal to the +z axis (rad): 0 is a line straight across the road, 90 degrees one beside it on the
    /// right, whose raised side is the right.
    angle: f32,
    /// A point of the line (physics x, z).
    through: (f32, f32),
}

impl Kerb {
    fn across(height: f32, bevel: f32, wall: bool) -> Self {
        Self { height, bevel, wall, angle: 0.0, through: (0.0, KERB_AT) }
    }

    /// The point `across` metres from the line along its normal and `along` metres along it, at height `y`.
    fn point(&self, across: f32, along: f32, y: f32) -> [f32; 3] {
        let (sin, cos) = self.angle.sin_cos();
        [self.through.0 + sin * across + cos * along, y, self.through.1 + cos * across - sin * along]
    }

    /// A slab from `a0` to `a1` across the line, at height `y`.
    fn slab(&self, y: f32, a0: f32, a1: f32) -> Strip {
        strip([self.point(a0, -SPAN, y), self.point(a0, SPAN, y), self.point(a1, -SPAN, y), self.point(a1, SPAN, y)])
    }
}

fn world(kerb: Kerb) -> CollisionWorld {
    let Kerb { height, bevel, wall, .. } = kerb;
    let mut strips = vec![kerb.slab(0.0, -REACH, 0.0), kerb.slab(height, bevel, REACH)];
    // The face of the rise: vertical, or with a bevel (its own tilted normal).
    strips.push(strip([
        kerb.point(0.0, -SPAN, 0.0),
        kerb.point(0.0, SPAN, 0.0),
        kerb.point(bevel, -SPAN, height),
        kerb.point(bevel, SPAN, height),
    ]));
    let barriers = match wall {
        true => {
            let (p0, p1) = (kerb.point(0.0, -SPAN, -1.0), kerb.point(0.0, SPAN, 3.0));
            let length = (p1[0] - p0[0]).hypot(p1[2] - p0[2]);
            vec![Barrier { p0, p1, surface: 0, flags: BARRIER_TWO_SIDED, inv_xz_len: 1.0 / length }]
        }
        false => vec![],
    };
    let mut world = CollisionWorld::new(None);
    world.insert(CollisionPack {
        section: 3,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            half_width: 300.0,
            half_height: 20.0,
            half_length: 300.0,
            translation: [0.0; 3],
            radius: 450.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article { strips, barriers, surfaces: vec![0], intermediate_object: 0, flags: 0 }],
        objects: vec![],
    });
    world
}

/// The example bike, or with `RIDE6_BIKE_TOML=<an imported bike's manifest>` that one (its box from the manifest).
fn bike_sim() -> CarSim {
    let Some(path) = std::env::var_os("RIDE6_BIKE_TOML") else {
        return CarSim::new(CarPhysics::from_bike(Arc::new(BikeSpec::example()), 0.0, BOUNDS), [0.32; 4]);
    };
    let text = std::fs::read_to_string(&path).expect("a readable manifest");
    let name = std::path::Path::new(&path).file_stem().unwrap_or_default().to_string_lossy().to_string();
    let manifest = blackbox_modding::content::parse_car(&name, &text).expect("a manifest");
    let spec = BikeSpec::from(manifest.bike.as_ref().expect("a bike"));
    let half = spec.chassis.half_dimensions;
    let bounds = CarBounds { half_dimensions: half, pivot: Vec3::new(0.0, half.y, 0.0) };
    CarSim::new(CarPhysics::from_bike(Arc::new(spec), 0.0, bounds), [0.32; 4])
}

/// What a ride at a kerb or a wall came to.
#[derive(Debug)]
struct Ride {
    fell: bool,
    /// How many steps had a contact of the world, and how many steps after the first one the bike fell.
    contacts: usize,
    fell_after: Option<usize>,
    /// How high the bike's origin got (m).
    top: f32,
}

/// A bike placed before the kerb at `speed` and held there by the throttle, with `steer` on the stick, for `seconds`.
fn ride(kerb: Kerb, speed: f32, steer: f32, seconds: f32) -> Ride {
    let collision = world(kerb);
    let surfaces = SurfaceTable::default();
    let ground = WorldGround { collision: &collision, surfaces: &surfaces };
    let mut props = PropWorld::default();
    let mut sim = bike_sim();
    let start = KERB_AT - 3.0 - speed * 1.2;
    assert!(sim.place_moving(&ground, Spawn { position: Vec3::new(start, 0.0, 2.0), heading: 0.0 }, speed));
    let mut out = Ride { fell: false, contacts: 0, fell_after: None, top: 0.0 };
    let mut first = None;
    for tick in 0..(seconds * 60.0) as usize {
        let throttle = ((speed - sim.telemetry().speed_mps) * 0.2 + 0.1).clamp(0.0, 0.5);
        let input = DriveInput { throttle, steer, ..DriveInput::default() };
        let impact = sim.step(&input, &ground, Some((&collision, &mut props)));
        out.contacts += usize::from(!impact.contacts.is_empty());
        if first.is_none() && !impact.contacts.is_empty() {
            first = Some(tick);
        }
        let ride = sim.ride.as_ref().expect("a bike");
        out.top = out.top.max(ride.bike.position().y);
        if ride.bike.fallen() {
            out.fell = true;
            out.fell_after = first.map(|first| tick - first);
            break;
        }
    }
    out
}

/// A wall across the road at `degrees` off the bike's heading: 90 is head-on.
fn wall(degrees: f32) -> Kerb {
    Kerb { angle: (90.0 - degrees).to_radians(), ..Kerb::across(0.0, 0.0, true) }
}

#[test]
fn a_bike_rides_over_a_low_kerb_at_any_speed_without_meeting_a_wall() {
    for height in [0.10, 0.15] {
        for bevel in [0.0, 0.05] {
            for degrees in [0.0_f32, 45.0] {
                for speed in [5.0, 15.0, 25.0, 35.0] {
                    let kerb = Kerb { angle: degrees.to_radians(), ..Kerb::across(height, bevel, false) };
                    let out = ride(kerb, speed, 0.0, 5.0);
                    let label = format!("{height} m, bevel {bevel}, {degrees} deg, {speed} m/s: {out:?}");
                    assert!(!out.fell && out.contacts == 0, "{label}");
                    assert!(out.top > height - 0.02, "the bike is on the kerb: {label}");
                }
            }
        }
    }
}

#[test]
fn a_bike_leaned_towards_a_kerb_beside_the_road_climbs_it_without_falling() {
    for steer in [0.4, 0.7] {
        for speed in [10.0, 20.0, 30.0] {
            let kerb =
                Kerb { height: 0.15, bevel: 0.0, wall: false, angle: 90.0_f32.to_radians(), through: (2.5, 0.0) };
            let out = ride(kerb, speed, steer, 5.0);
            assert!(!out.fell && out.contacts == 0, "steer {steer}, {speed} m/s: {out:?}");
            assert!(out.top > 0.14, "steer {steer}, {speed} m/s: {out:?}");
        }
    }
}

#[test]
fn a_glancing_touch_of_a_wall_does_not_throw_the_bike() {
    // The closing speed of every one of these is under 9 m/s.
    for (degrees, speed) in [(10.0, 8.0), (10.0, 25.0), (20.0, 15.0), (20.0, 25.0), (30.0, 15.0), (60.0, 8.0)] {
        let out = ride(wall(degrees), speed, 0.0, 4.0);
        assert!(out.contacts > 0, "{degrees} deg, {speed} m/s met the wall");
        assert!(!out.fell, "{degrees} deg, {speed} m/s: {out:?}");
    }
}

#[test]
fn a_wall_at_speed_throws_the_bike_at_the_touch() {
    for (degrees, speed) in [(90.0, 20.0), (90.0, 35.0), (60.0, 25.0), (60.0, 35.0), (30.0, 35.0)] {
        let out = ride(wall(degrees), speed, 0.0, 4.0);
        assert!(out.fell, "{degrees} deg, {speed} m/s: {out:?}");
        assert!(out.fell_after.is_some_and(|steps| steps <= 2), "at the touch, not later: {out:?}");
    }
}

#[test]
fn a_bike_ridden_slowly_into_a_wall_stops_and_stays_up() {
    let out = ride(wall(90.0), 5.0, 0.0, 5.0);
    assert!(out.contacts > 0 && !out.fell, "{out:?}");
}

/// The installed world's roads ridden into their edges, for the numbers of `ride6-feel-round10.md` (section 4): from 40 places on its
/// roads, 16 rides each (the stick at +-0.15 and +-0.3 for `HOLD` steps from the 60th, 8 to 28 m/s), counting the falls by how fast
/// the first wall contact closed in. `RIDE6_BIKE_TOML=<a bike's manifest>` rides an imported bike; `NFSMW_GAME_DIR` names the install.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn survey_installed_roadsides() {
    use crate::scenes::world::{road, space};
    use game_install::GameDir;
    use nfsmw_data::car::physics::PhysicsData;
    use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let centre = index.centre();
    let physics = PhysicsData::load(&dir).unwrap();
    let mut collision = CollisionWorld::new(index.collision_grid.clone());
    for section in index.sections.iter().filter(|s| s.is_spatial() && s.distance_to(centre[0], centre[1]) <= 1200.0) {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        blackbox_collision::read_collision_packs(&bytes).unwrap().into_iter().for_each(|pack| collision.insert(pack));
    }
    let ground = WorldGround { collision: &collision, surfaces: &physics.surfaces };
    let probe = |x: f32, y: f32| {
        let [px, _, pz] = space::to_physics(Vec3::new(x, y, 0.0));
        let top = collision.ground(px, pz, 1000.0, -1000.0)?.point[1] + 1.0;
        road::probe(&collision, x, y, top)
    };
    let hold: usize = std::env::var("HOLD").ok().and_then(|h| h.parse().ok()).unwrap_or(60);
    let spots: Vec<_> = (0..40)
        .filter_map(|i| {
            let (a, r) = (i as f32 * 0.157, 150.0 + 20.0 * i as f32);
            road::find([centre[0] + r * a.cos(), centre[1] + r * a.sin()], probe)
        })
        .collect();
    let mut buckets = std::collections::BTreeMap::<i32, (usize, usize)>::new();
    for spot in &spots {
        for steer in [0.15_f32, -0.15, 0.3, -0.3] {
            for speed in [8.0_f32, 14.0, 20.0, 28.0] {
                let (mut sim, mut props) = (bike_sim(), PropWorld::default());
                if !sim.place_moving(&ground, *spot, speed) {
                    continue;
                }
                let mut closing = None;
                for tick in 0..360 {
                    let throttle = ((speed - sim.telemetry().speed_mps) * 0.2 + 0.1).clamp(0.0, 0.5);
                    let steer = if (60..60 + hold).contains(&tick) { steer } else { 0.0 };
                    let input = DriveInput { throttle, steer, ..DriveInput::default() };
                    let impact = sim.step(&input, &ground, Some((&collision, &mut props)));
                    closing = closing.or_else(|| impact.visuals.first().map(|c| (-c.velocity.dot(c.normal)).max(0.0)));
                    if sim.ride.as_ref().is_some_and(|ride| ride.bike.fallen()) {
                        break;
                    }
                }
                let Some(closing) = closing else { continue };
                let entry = buckets.entry((closing / 3.0) as i32 * 3).or_default();
                entry.0 += 1;
                entry.1 += usize::from(sim.ride.as_ref().is_some_and(|ride| ride.bike.fallen()));
            }
        }
    }
    for (closing, (rides, falls)) in buckets {
        println!("first contact closing at {closing:>2} to {:>2} m/s: {rides:>3} rides, {falls:>3} fell", closing + 3);
    }
}

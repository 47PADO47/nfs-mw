//! The values a window keeps between rays (`crate::face_cache`) must give exactly the hits of walking the grid per ray:
//! the same distance, point, normal and face bits, over rotated and tilted instances, bumpy strips, flagged triangles,
//! scenery groups and an ungridded pack, with the window kept across ticks and world changes.

use std::collections::BTreeMap;

use super::grid::{grid_payload, node};
use super::synth::*;
use super::window::Lcg;
use crate::{
    CandidateSet, CollisionPack, CollisionWorld, FLAG_Y_NOT_UP, GROUP_EXCLUSION, Hit, HitKind, InstanceRef, RayOptions,
    SURFACE_NO_GROUND,
};

const SURFACES: [u32; 3] = [0x11, 0x22, 0x33];
/// Instances of each pack, on a 6 x 4 lattice over the 4 x 3 grid of 64 m cells whose corner is (-100, 0, -50).
const INSTANCES: usize = 24;
const HALF: [f32; 3] = [8.0, 3.0, 8.0];

/// A strip of `columns` vertex pairs along local x over [-8, 8], `half_width` either side of `z`, at heights jittered
/// around `y`, with surfaces and triangle flags drawn from `rng` (some not ground, some role-excluded).
fn bumpy_strip(rng: &mut Lcg, z: f32, y: f32, half_width: f32, flags: u16) -> StripSpec {
    let columns = 6;
    let mut verts = Vec::new();
    for i in 0..columns {
        let x = -8.0 + 16.0 * i as f32 / (columns - 1) as f32;
        for side in [-1.0, 1.0] {
            let height = rng.range(-0.6, 0.6);
            let surface = (rng.next() * 3.0) as u8;
            let flag = [0, 0, 0, SURFACE_NO_GROUND, 0x40, 0x01][(rng.next() * 6.0) as usize];
            let fixed = |v: f32| (v * 128.0).round() as i16;
            verts.push(vert(fixed(x), fixed(height), fixed(side * half_width), surface, flag));
        }
    }
    StripSpec { center: [0.0, y, z], radius: 10 * 16, flags, verts }
}

fn articles(rng: &mut Lcg) -> Vec<Vec<u8>> {
    let mut ground: Vec<StripSpec> = [-6.0, -2.0, 2.0, 6.0].map(|z| bumpy_strip(rng, z, 0.0, 2.0, 0)).into();
    // A deck above part of the ground, and a strip only role-excluded rays skip.
    ground.push(bumpy_strip(rng, 0.0, 1.5, 3.0, 0));
    ground.push(bumpy_strip(rng, 4.0, 0.8, 2.0, 0x40));
    let wall = BarrierSpec { p0: [2.0, -1.0, -6.0], p1: [3.0, 3.0, 6.0], surface: 1, flags: 0 };
    let ramp = vec![bumpy_strip(rng, 0.0, 0.5, 8.0, 0), bumpy_strip(rng, 3.0, 2.0, 2.0, 0)];
    vec![article(&ground, &[wall], &SURFACES), article(&ramp, &[], &SURFACES)]
}

/// Rows of a rotation by `yaw` about y then `tilt` about x: (row x, row z).
fn rows(yaw: f32, tilt: f32) -> ([f32; 3], [f32; 3]) {
    let (sy, cy) = yaw.sin_cos();
    let (st, ct) = tilt.sin_cos();
    ([cy, 0.0, -sy], [sy * ct, -st, cy * ct])
}

fn specs(rng: &mut Lcg, grouped: bool) -> Vec<InstanceSpec> {
    (0..INSTANCES)
        .map(|i| {
            let position = [
                -90.0 + (i % 6) as f32 * 32.0 + rng.range(-4.0, 4.0),
                rng.range(-1.0, 1.0),
                -40.0 + (i / 6) as f32 * 55.0 + rng.range(-4.0, 4.0),
            ];
            let tilted = i % 4 == 0;
            let (row_x, row_z) =
                rows(rng.range(0.0, std::f32::consts::TAU), if tilted { rng.range(-0.3, 0.3) } else { 0.0 });
            let mut spec = InstanceSpec::upright(position, HALF, (i % 3 == 2) as u16);
            spec.row_x = row_x;
            spec.row_z = row_z;
            spec.flags = if tilted { FLAG_Y_NOT_UP } else { 0 };
            if grouped && i % 5 == 1 {
                spec.group = 5;
            }
            if grouped && i % 7 == 3 {
                spec.flags |= 0x40;
            }
            spec
        })
        .collect()
}

/// Every cell the xz square around each instance touches lists it, in an unsorted order, plus a pack never loaded.
fn grid(sections: &[(u16, &[InstanceSpec])]) -> crate::Grid {
    let cell = |v: f32, min: f32, count: i32| (((v - min) / 64.0).floor() as i32).clamp(0, count - 1) as usize;
    let mut lists: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    for &(section, specs) in sections.iter().rev() {
        for (i, spec) in specs.iter().enumerate().rev() {
            let [x, _, z] = spec.position;
            let reach = spec.radius + 1.0;
            for row in cell(z - reach, -50.0, 4)..=cell(z + reach, -50.0, 4) {
                for col in cell(x - reach, -100.0, 3)..=cell(x + reach, -100.0, 3) {
                    lists.entry(row * 3 + col).or_default().push(InstanceRef::new(section, i as u16).0);
                }
            }
        }
    }
    lists.entry(5).or_default().push(InstanceRef::new(9, 0).0);
    let nodes: Vec<Vec<u8>> =
        lists.iter().map(|(&index, list)| node(index as u16, [list.as_slice(), &[], &[], &[]])).collect();
    crate::Grid::parse(&grid_payload(&nodes)).unwrap()
}

/// The world, and pack 8's bytes to put back after removing it.
fn world() -> (CollisionWorld, Vec<u8>) {
    let mut rng = Lcg(0xFACE);
    let articles = articles(&mut rng);
    let plain = specs(&mut Lcg(0x5EC7), false);
    // The same placements again in another section (equal distances), some grouped or role-flagged.
    let grouped = specs(&mut Lcg(0x5EC7), true);
    let mut world = CollisionWorld::new(Some(grid(&[(7, &plain), (8, &grouped)])));
    world.insert(CollisionPack::parse(&pack(7, &articles, &plain)).unwrap());
    let eight = pack(8, &articles, &grouped);
    world.insert(CollisionPack::parse(&eight).unwrap());
    let mut loose = InstanceSpec::upright([-60.0, 0.3, 20.0], HALF, 1);
    (loose.row_x, loose.row_z) = rows(0.7, 0.2);
    loose.flags = FLAG_Y_NOT_UP;
    world.insert_ungridded(CollisionPack::parse(&pack(3000, &articles, &[loose])).unwrap());
    (world, eight)
}

/// The tyre rays' options (`WorldGround`), then the other kinds a window meets between them.
pub(super) fn options() -> [RayOptions; 5] {
    [
        RayOptions {
            barriers: false,
            skip_groups: true,
            exclude: u32::from(SURFACE_NO_GROUND) | u32::from(GROUP_EXCLUSION),
            ..RayOptions::default()
        },
        RayOptions::default(),
        RayOptions { barriers: false, ..RayOptions::default() },
        RayOptions { exclude: 0x40, skip_groups: true, ..RayOptions::default() },
        RayOptions { faces: false, ..RayOptions::default() },
    ]
}

pub(super) type Bits = (u32, [u32; 3], [u32; 3], HitKind, bool, u32, u8, u8, u32, usize);

/// Every field of a hit, floats as their bits.
pub(super) fn bits(hit: Option<Hit>) -> Option<Bits> {
    let hit = hit?;
    Some((
        hit.t.to_bits(),
        hit.point.map(f32::to_bits),
        hit.normal.map(f32::to_bits),
        hit.kind,
        hit.front_facing,
        hit.surface_hash,
        hit.surface_index,
        hit.surface_flags,
        hit.section,
        hit.instance,
    ))
}

/// What a car casts in one tick at `centre` heading `yaw`: the eight body-corner rays straight down from their lift
/// (`RigidBody::ground_contact`), then eight wall probes.
fn car_rays(rng: &mut Lcg, centre: [f32; 3], yaw: f32) -> Vec<([f32; 3], [f32; 3])> {
    let (s, c) = yaw.sin_cos();
    let half = [1.0, 0.7, 2.4];
    let mut rays = Vec::new();
    for corner in [[-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], [1.0, -1.0, -1.0], [-1.0, -1.0, -1.0]].into_iter().chain([
        [-1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, -1.0],
        [-1.0, 1.0, -1.0],
    ]) {
        let [x, y, z] = [half[0] * corner[0], half[1] * corner[1], half[2] * corner[2]];
        let point = [centre[0] + c * x + s * z, centre[1] + y, centre[2] - s * x + c * z];
        let lift = rng.range(0.25, 3.0);
        let origin = [point[0], point[1] + lift, point[2]];
        rays.push((origin, [origin[0], origin[1] - lift, origin[2]]));
    }
    for k in 0..8 {
        let angle = yaw + k as f32 * std::f32::consts::FRAC_PI_4;
        rays.push((centre, [centre[0] + 3.0 * angle.cos(), centre[1] - 0.3, centre[2] + 3.0 * angle.sin()]));
    }
    rays
}

/// Every predicate call, in order, then the winner, of a cast whose predicate rejects some faces and barriers.
fn traced(cast: impl FnOnce(&mut dyn FnMut(&Hit) -> bool) -> Option<Hit>) -> (Vec<Option<Bits>>, Option<Bits>) {
    let mut seen = Vec::new();
    let best = cast(&mut |hit: &Hit| {
        seen.push(bits(Some(*hit)));
        (hit.kind == HitKind::Face && hit.surface_index != 2) || seen.len().is_multiple_of(3)
    });
    (seen, bits(best))
}

/// Hits found, and hits on tilted instances, so the test knows it tested something.
#[derive(Default)]
struct Tally {
    hits: usize,
    tilted: usize,
}

/// Forty cars driving six ticks each near the instances: every ray through the car's kept window, the world's shared
/// window and the filtered cast must equal the grid walk, bit for bit and predicate call for predicate call.
fn drive(world: &CollisionWorld, seed: u64, window: &mut CandidateSet, tally: &mut Tally) {
    let mut rng = Lcg(seed);
    for _ in 0..40 {
        let base = specs(&mut Lcg(0x5EC7), false)[(rng.next() * INSTANCES as f32) as usize].position;
        let mut centre =
            [base[0] + rng.range(-9.0, 9.0), base[1] + rng.range(-0.5, 1.5), base[2] + rng.range(-9.0, 9.0)];
        let mut yaw = rng.range(0.0, std::f32::consts::TAU);
        for _ in 0..6 {
            for (from, to) in car_rays(&mut rng, centre, yaw) {
                for opts in options() {
                    let walked = world.ray_cast_walk(from, to, &opts, |_| true);
                    assert_eq!(
                        bits(world.ray_cast_in(from, to, &opts, window)),
                        bits(walked),
                        "{from:?} {to:?} {opts:?}"
                    );
                    assert_eq!(bits(world.ray_cast(from, to, &opts)), bits(walked), "shared {from:?} {to:?}");
                    let filtered = traced(|accept| world.ray_cast_filtered(from, to, &opts, accept));
                    assert_eq!(filtered, traced(|accept| world.ray_cast_walk(from, to, &opts, accept)));
                    let Some(hit) = walked else { continue };
                    tally.hits += 1;
                    tally.tilted += usize::from(hit.instance % 4 == 0 || hit.section == 3000);
                }
            }
            centre = [centre[0] + 0.4 * yaw.sin(), centre[1], centre[2] + 0.4 * yaw.cos()];
            yaw += rng.range(-0.1, 0.1);
        }
    }
}

#[test]
fn kept_faces_match_the_grid_walk_bit_for_bit() {
    let (mut world, eight) = world();
    let mut window = CandidateSet::default();
    let mut tally = Tally::default();
    drive(&world, 0xC0FE, &mut window, &mut tally);
    assert!(window.kept_triangles() > 0, "the car's window keeps triangles between its rays");

    // A kept window follows every kind of world change.
    world.set_group_enabled(5, true);
    drive(&world, 0xC0FF, &mut window, &mut tally);
    world.remove(7);
    drive(&world, 0xC100, &mut window, &mut tally);
    world.insert(CollisionPack::parse(&eight).unwrap());
    world.set_group_enabled(5, false);
    drive(&world, 0xC101, &mut window, &mut tally);
    world.remove_ungridded(3000);
    drive(&world, 0xC102, &mut window, &mut tally);

    assert!(tally.hits > 10_000, "{} hits", tally.hits);
    assert!(tally.tilted > 1_000, "{} hits on tilted instances", tally.tilted);
}

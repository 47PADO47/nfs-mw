//! The candidate-window and candidate-set paths must return exactly what walking the grid per ray returns.

use std::collections::BTreeMap;

use super::grid::{grid_payload, node};
use super::synth::*;
use crate::{CandidateSet, CollisionPack, CollisionWorld, Grid, Hit, HitKind, InstanceRef, RayOptions};

const SURFACE: u32 = 7;
const GRID_COLS: usize = 3;

/// Deterministic pseudo-random numbers in `[0, 1)`.
pub(super) struct Lcg(pub(super) u64);

impl Lcg {
    pub(super) fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    pub(super) fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next()
    }
}

fn articles() -> Vec<Vec<u8>> {
    let wall = BarrierSpec { p0: [1.0, -1.0, -4.0], p1: [1.0, 3.0, 4.0], surface: 0, flags: 2 };
    vec![article(&[ground_strip(0)], &[wall], &[SURFACE]), article(&[ground_strip(0)], &[], &[SURFACE])]
}

/// Twelve instances spread over the 4 x 3 grid of 64 m cells whose corner is (-100, 0, -50).
fn specs(group_of_last: u16) -> Vec<InstanceSpec> {
    (0..12)
        .map(|i| {
            let position = [-90.0 + (i % 4) as f32 * 55.0, 0.0, -40.0 + (i / 4) as f32 * 70.0];
            let mut spec = InstanceSpec::upright(position, [4.0, 1.0, 4.0], (i % 2) as u16);
            if i == 11 {
                spec.group = group_of_last;
            }
            spec
        })
        .collect()
}

fn cell_of(position: [f32; 3]) -> (usize, usize) {
    (((position[2] + 50.0) / 64.0) as usize, ((position[0] + 100.0) / 64.0) as usize)
}

/// A grid listing every instance of `sections` in its cell, every third one also in the next column (a straddler),
/// in an unsorted order, plus a reference to a pack that is never loaded.
fn grid(sections: &[u16]) -> Grid {
    let mut lists: BTreeMap<usize, Vec<u32>> = BTreeMap::new();
    for &section in sections.iter().rev() {
        for (i, spec) in specs(0).iter().enumerate().rev() {
            let (row, col) = cell_of(spec.position);
            let reference = InstanceRef::new(section, i as u16).0;
            lists.entry(row * GRID_COLS + col).or_default().push(reference);
            if i.is_multiple_of(3) && col + 1 < GRID_COLS {
                lists.entry(row * GRID_COLS + col + 1).or_default().push(reference);
            }
        }
    }
    lists.entry(0).or_default().push(InstanceRef::new(9, 0).0);
    let nodes: Vec<Vec<u8>> =
        lists.iter().map(|(&index, list)| node(index as u16, [list.as_slice(), &[], &[], &[]])).collect();
    Grid::parse(&grid_payload(&nodes)).unwrap()
}

fn world() -> CollisionWorld {
    let mut world = CollisionWorld::new(Some(grid(&[7, 8])));
    world.insert(CollisionPack::parse(&pack(7, &articles(), &specs(0))).unwrap());
    // Same geometry again in another section (equal distances), one instance in scenery group 5.
    world.insert(CollisionPack::parse(&pack(8, &articles(), &specs(5))).unwrap());
    let wall = BarrierSpec { p0: [-2.0, -1.0, -4.0], p1: [-2.0, 3.0, 4.0], surface: 0, flags: 0 };
    let loose = [article(&[], &[wall], &[SURFACE])];
    let spec = InstanceSpec::upright([-60.0, 0.0, 20.0], [4.0, 1.0, 4.0], 0);
    world.insert_ungridded(CollisionPack::parse(&pack(3000, &loose, &[spec])).unwrap());
    world
}

fn options() -> [RayOptions; 5] {
    [
        RayOptions::default(),
        RayOptions { barriers: false, ..RayOptions::default() },
        RayOptions { faces: false, ..RayOptions::default() },
        RayOptions { skip_groups: true, ..RayOptions::default() },
        RayOptions { exclude: 0xC0, skip_groups: true, ..RayOptions::default() },
    ]
}

/// Every predicate call, in order, then the winner: the fingerprint two cast paths must share.
type Trace = (Vec<(u32, usize, u32)>, Option<Hit>);

fn traced(cast: impl FnOnce(&mut dyn FnMut(&Hit) -> bool) -> Option<Hit>) -> Trace {
    let mut seen = Vec::new();
    // Rejects steep faces and every other barrier hit so the predicate has a say.
    let mut accept = |hit: &Hit| {
        seen.push((hit.section, hit.instance, hit.t.to_bits()));
        hit.kind == HitKind::Face || seen.len().is_multiple_of(2)
    };
    let best = cast(&mut accept);
    (seen, best)
}

fn check(world: &CollisionWorld, set: Option<&CandidateSet>, from: [f32; 3], to: [f32; 3]) {
    for opts in options() {
        let walked = traced(|accept| world.ray_cast_walk(from, to, &opts, accept));
        let windowed = traced(|accept| world.ray_cast_filtered(from, to, &opts, accept));
        assert_eq!(windowed, walked, "window {from:?} -> {to:?} {opts:?}");
        let plain = world.ray_cast_walk(from, to, &opts, |_| true);
        assert_eq!(world.ray_cast(from, to, &opts), plain, "plain {from:?} -> {to:?}");
        let Some(set) = set else { continue };
        let shared = traced(|accept| world.ray_cast_candidates_filtered(from, to, &opts, set, accept));
        assert_eq!(shared, walked, "set {from:?} -> {to:?} {opts:?}");
        assert_eq!(world.ray_cast_candidates(from, to, &opts, set), plain);
    }
}

fn random_point(rng: &mut Lcg) -> [f32; 3] {
    [rng.range(-120.0, 110.0), rng.range(-3.0, 3.0), rng.range(-70.0, 230.0)]
}

fn rays(world: &CollisionWorld, rng: &mut Lcg, count: usize) {
    for _ in 0..count {
        let centre = random_point(rng);
        // A car's sixteen rays: eight wall probes from the centre, eight corner rays straight down.
        let mut set = CandidateSet::default();
        let reach = 3.0;
        let low = [centre[0] - reach, centre[1] - 2.0, centre[2] - reach];
        let high = [centre[0] + reach, centre[1] + 2.0, centre[2] + reach];
        let gathered = world.candidates_in(low, high, &mut set);
        assert!(gathered);
        for k in 0..8 {
            let angle = k as f32 * std::f32::consts::FRAC_PI_4;
            let probe = [centre[0] + reach * angle.cos(), centre[1], centre[2] + reach * angle.sin()];
            check(world, Some(&set), centre, probe);
            let corner = [centre[0] + 1.5 * angle.sin(), centre[1] + 1.0, centre[2] + 2.5 * angle.cos()];
            check(world, Some(&set), corner, [corner[0], corner[1] - 2.0, corner[2]]);
        }
        // Long and arbitrary rays, which walk the grid.
        check(world, None, random_point(rng), random_point(rng));
        // Straight down over the whole depth, as ground queries do.
        check(world, Some(&set), [centre[0], 8.0, centre[2]], [centre[0], -8.0, centre[2]]);
    }
}

#[test]
fn window_and_candidate_sets_match_the_per_ray_grid_walk() {
    let mut world = world();
    let mut rng = Lcg(0x5EED);
    rays(&world, &mut rng, 60);

    // Every kind of world change moves the revision, so a kept window or set is rebuilt rather than reused.
    world.set_group_enabled(5, true);
    rays(&world, &mut rng, 20);
    world.remove(8);
    rays(&world, &mut rng, 20);
    world.insert(CollisionPack::parse(&pack(8, &articles(), &specs(5))).unwrap());
    world.set_group_enabled(5, false);
    rays(&world, &mut rng, 20);
    world.remove_ungridded(3000);
    rays(&world, &mut rng, 20);
}

#[test]
fn a_stale_candidate_set_falls_back_to_the_current_world() {
    let mut world = world();
    let mut set = CandidateSet::default();
    assert!(world.candidates_in([-90.0, 0.0, -45.0], [-80.0, 0.0, -35.0], &mut set));
    assert!(!set.is_empty());
    let (from, to) = ([-90.0, 5.0, -40.0], [-90.0, -5.0, -40.0]);
    let before = world.ray_cast_candidates(from, to, &RayOptions::default(), &set).unwrap();
    assert!((before.t - 0.5).abs() < 1e-5);
    world.remove(7);
    world.remove(8);
    assert!(world.ray_cast_candidates(from, to, &RayOptions::default(), &set).is_none(), "the set is stale");
    // A ray outside the gathered cells also falls back instead of missing its candidates.
    let (from, to) = ([60.0, 5.0, 100.0], [60.0, -5.0, 100.0]);
    let fresh = world.ray_cast(from, to, &RayOptions::default());
    assert_eq!(world.ray_cast_candidates(from, to, &RayOptions::default(), &set), fresh);
}

#[test]
fn candidate_sets_refuse_what_they_cannot_cover() {
    let world = world();
    let mut set = CandidateSet::default();
    assert!(!world.candidates_in([f32::NAN, 0.0, 0.0], [1.0; 3], &mut set));
    assert!(set.is_empty());
    assert!(!CollisionWorld::new(None).candidates_in([0.0; 3], [1.0; 3], &mut set));
    // The whole 4 x 3 grid is 12 cells, well inside the 64-cell limit.
    assert!(world.candidates_in([-1e9, 0.0, -1e9], [1e9, 0.0, 1e9], &mut set));
    assert!(!set.is_empty());
}

#[test]
fn a_predicate_that_casts_again_sees_the_same_world() {
    let world = world();
    let opts = RayOptions::default();
    let (from, to) = ([-90.0, 5.0, -40.0], [-90.0, -5.0, -40.0]);
    let expected = world.ray_cast_walk(from, to, &opts, |_| true);
    let nested = world.ray_cast_filtered(from, to, &opts, |_| {
        let inner = world.ray_cast([-35.0, 5.0, -40.0], [-35.0, -5.0, -40.0], &opts);
        inner.is_some()
    });
    assert_eq!(nested, expected);
    assert_eq!(world.ray_cast(from, to, &opts), expected);
}

#[test]
fn sections_beyond_the_grid_range_and_packs_keep_section_order() {
    let mut world = CollisionWorld::new(None);
    let tile = || vec![article(&[ground_strip(0)], &[], &[SURFACE])];
    let spec = || [InstanceSpec::upright([0.0; 3], [4.0; 3], 0)];
    for section in [70_000u32, 5, 3, 65_535] {
        world.insert(CollisionPack::parse(&pack(section, &tile(), &spec())).unwrap());
    }
    assert_eq!(world.pack_count(), 4);
    for section in [70_000u32, 5, 3, 65_535] {
        assert_eq!(world.pack(section).map(|p| p.section), Some(section));
    }
    assert!(world.pack(4).is_none() && world.pack(70_001).is_none());
    // Equal distances: the lowest section wins, as the ordered map iterated before.
    let hit = world.ray_cast([0.0, 5.0, 0.0], [0.0, -5.0, 0.0], &RayOptions::default()).unwrap();
    assert_eq!(hit.section, 3);
    assert!(world.remove(3).is_some() && world.remove(3).is_none());
    assert_eq!(world.pack(5).map(|p| p.section), Some(5), "slots are rebuilt after a removal");
    assert_eq!(world.ray_cast([0.0, 5.0, 0.0], [0.0, -5.0, 0.0], &RayOptions::default()).unwrap().section, 5);
}

#[test]
fn resident_answers_follow_the_revision_and_repeat() {
    let mut world = CollisionWorld::new(Some(grid(&[7, 8])));
    let point = [-90.0, 0.0, -40.0];
    // Section 9 is listed in cell 0 and never loaded.
    for _ in 0..3 {
        assert_eq!(world.resident_near(point, 5.0), Some(false));
    }
    world.insert(CollisionPack::parse(&pack(9, &articles(), &specs(0))).unwrap());
    world.insert(CollisionPack::parse(&pack(7, &articles(), &specs(0))).unwrap());
    world.insert(CollisionPack::parse(&pack(8, &articles(), &specs(0))).unwrap());
    for _ in 0..3 {
        assert_eq!(world.resident_near(point, 5.0), Some(true));
    }
    world.remove(8);
    assert_eq!(world.resident_near(point, 5.0), Some(false), "a removal invalidates the kept answer");
}

#[test]
fn group_set_matches_a_plain_set() {
    let mut world = CollisionWorld::new(None);
    let groups = [0u16, 1, 5, 63, 64, 65, 1000, 65_535];
    for &g in &groups {
        assert!(!world.group_enabled(g));
        world.set_group_enabled(g, true);
        assert!(world.group_enabled(g));
    }
    let revision = world.revision();
    world.set_group_enabled(5, true);
    assert_eq!(world.revision(), revision, "re-enabling changes nothing");
    world.set_group_enabled(64, false);
    assert!(!world.group_enabled(64) && world.group_enabled(63) && world.group_enabled(65));
    world.set_group_enabled(2, false);
    assert_ne!(world.revision(), revision);
}

/// Short rays around random points, as a car casts them, each answered by `cast`: the hits must equal the grid walk.
fn car_rays(world: &CollisionWorld, seed: u64, mut cast: impl FnMut([f32; 3], [f32; 3], &RayOptions) -> Option<Hit>) {
    let mut rng = Lcg(seed);
    for _ in 0..40 {
        let centre = random_point(&mut rng);
        for k in 0..8 {
            let angle = k as f32 * std::f32::consts::FRAC_PI_4;
            let probe = [centre[0] + 3.0 * angle.cos(), centre[1], centre[2] + 3.0 * angle.sin()];
            let down = [centre[0] + 1.5 * angle.sin(), 8.0, centre[2] + 2.5 * angle.cos()];
            for opts in options() {
                assert_eq!(cast(centre, probe, &opts), world.ray_cast_walk(centre, probe, &opts, |_| true));
                let bottom = [down[0], -8.0, down[2]];
                assert_eq!(cast(down, bottom, &opts), world.ray_cast_walk(down, bottom, &opts, |_| true));
            }
        }
        // A long ray, which walks the grid whatever window it is given.
        let (from, to) = (random_point(&mut rng), random_point(&mut rng));
        assert_eq!(cast(from, to, &options()[0]), world.ray_cast_walk(from, to, &options()[0], |_| true));
    }
}

#[test]
fn a_caller_window_matches_the_grid_walk_and_survives_world_changes() {
    let mut world = world();
    let mut window = CandidateSet::default();
    car_rays(&world, 0xCA5, |from, to, opts| world.ray_cast_in(from, to, opts, &mut window));
    world.set_group_enabled(5, true);
    world.remove(7);
    car_rays(&world, 0xCA6, |from, to, opts| world.ray_cast_in(from, to, opts, &mut window));
}

#[test]
fn threads_cast_at_once_with_their_own_windows_and_the_shared_one() {
    let world = world();
    let resident = world.resident_near([-90.0, 0.0, -40.0], 5.0);
    std::thread::scope(|scope| {
        for seed in 0..4u64 {
            let world = &world;
            scope.spawn(move || {
                let mut window = CandidateSet::default();
                car_rays(world, 0x100 + seed, |from, to, opts| world.ray_cast_in(from, to, opts, &mut window));
                car_rays(world, 0x200 + seed, |from, to, opts| world.ray_cast(from, to, opts));
                assert_eq!(world.resident_near([-90.0, 0.0, -40.0], 5.0), resident);
            });
        }
    });
}

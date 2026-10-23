//! Against a real install (set `NFSMW_GAME_DIR`): a car's window, keeping its instances' values between rays, gives
//! the grid walk's hits bit for bit over the whole city.

use super::face_cache::{bits, options};
use super::install::{game_file, packs};
use super::window::Lcg;
use crate::{CandidateSet, CollisionWorld, Grid};

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_kept_window_matches_the_grid_walk() {
    let Some(packs) = packs() else { return };
    let grid = Grid::read(&game_file("TRACKS/L2RA.BUN").unwrap()).expect("grid").expect("a grid chunk");
    let mut world = CollisionWorld::new(Some(grid));
    for pack in &packs {
        world.insert(pack.clone());
    }
    let mut rng = Lcg(0x0C17);
    let mut window = CandidateSet::default();
    let (mut casts, mut hits) = (0usize, 0usize);
    // A car parked over the middle of each strip of every seventh instance, casting its corner rays and probes as it
    // creeps forward for a few ticks.
    for pack in &packs {
        for (i, inst) in pack.instances.iter().enumerate().step_by(7) {
            let Some(article) = pack.article_of(i) else { continue };
            for strip in article.strips.iter().take(4).filter(|strip| strip.triangle_count() > 0) {
                let tri = strip.triangle(0);
                let middle = [0, 1, 2].map(|k| (tri.pts[0][k] + tri.pts[1][k] + tri.pts[2][k]) / 3.0);
                let mut centre = inst.to_world(middle);
                centre[1] += rng.range(0.2, 0.9);
                let yaw = rng.range(0.0, std::f32::consts::TAU);
                for _ in 0..3 {
                    for (from, to) in corner_rays(&mut rng, centre, yaw) {
                        for opts in options() {
                            let walked = world.ray_cast_walk(from, to, &opts, |_| true);
                            assert_eq!(bits(world.ray_cast_in(from, to, &opts, &mut window)), bits(walked));
                            casts += 1;
                            hits += usize::from(walked.is_some());
                        }
                    }
                    centre = [centre[0] + 0.5 * yaw.sin(), centre[1], centre[2] + 0.5 * yaw.cos()];
                }
            }
        }
    }
    assert!(hits > casts / 10, "{hits} hits of {casts} casts");
}

/// The eight body-corner rays of a car at `centre` heading `yaw`, straight down from their lift.
fn corner_rays(rng: &mut Lcg, centre: [f32; 3], yaw: f32) -> Vec<([f32; 3], [f32; 3])> {
    let (s, c) = yaw.sin_cos();
    let mut rays = Vec::new();
    for (x, y, z) in [(-1.0, -0.7, 2.4), (1.0, -0.7, 2.4), (1.0, -0.7, -2.4), (-1.0, -0.7, -2.4)].into_iter().chain([
        (-1.0, 0.7, 2.4),
        (1.0, 0.7, 2.4),
        (1.0, 0.7, -2.4),
        (-1.0, 0.7, -2.4),
    ]) {
        let lift = rng.range(0.25, 3.0);
        let origin = [centre[0] + c * x + s * z, centre[1] + y + lift, centre[2] - s * x + c * z];
        rays.push((origin, [origin[0], origin[1] - lift, origin[2]]));
    }
    rays
}

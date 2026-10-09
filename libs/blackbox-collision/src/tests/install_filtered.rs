//! Install-backed evidence for candidate eligibility. No game bytes are fixtures.

use crate::math::{add, scale, sub};
use crate::{BARRIER_TWO_SIDED, CollisionWorld, GROUP_EXCLUSION, Grid, Hit, HitKind, RayOptions, read_collision_packs};

fn accept_wall(hit: &Hit) -> bool {
    if hit.kind == HitKind::Face && hit.normal[1].abs() > 0.7 {
        return false;
    }
    hit.kind != HitKind::Barrier || hit.front_facing || hit.surface_flags & BARRIER_TWO_SIDED != 0
}

fn file(path: &str) -> Vec<u8> {
    let dir = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR");
    std::fs::read(std::path::Path::new(&dir).join(path)).unwrap()
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn road_height_backfaces_do_not_hide_an_eligible_barrier() {
    let packs = read_collision_packs(&file("TRACKS/STREAML2RA.BUN")).unwrap();
    let grid = Grid::read(&file("TRACKS/L2RA.BUN")).unwrap().unwrap();
    let mut world = CollisionWorld::new(Some(grid));
    for pack in &packs {
        world.insert(pack.clone());
    }
    let options = RayOptions { exclude: u32::from(GROUP_EXCLUSION), ..RayOptions::default() };
    let ground_options = RayOptions { barriers: false, exclude: options.exclude | 8, ..options };
    let mut tested = 0;
    let mut witnessed = 0;
    'packs: for pack in &packs {
        for (index, inst) in pack.instances.iter().enumerate() {
            if inst.group != 0 {
                continue;
            }
            for barrier in &pack.article_of(index).unwrap().barriers {
                if barrier.flags & BARRIER_TWO_SIDED != 0 {
                    continue;
                }
                let middle = inst.to_world(scale(add(barrier.p0, barrier.p1), 0.5));
                let normal = inst.dir_to_world(barrier.normal());
                let mut from = sub(middle, scale(normal, 0.5));
                let mut to = add(middle, scale(normal, 1.5));
                let ground = world.ray_cast([from[0], 1000.0, from[2]], [from[0], -500.0, from[2]], &ground_options);
                let Some(ground) = ground else { continue };
                // Body-height horizontal probes, not the midpoint of a wall whose authored height
                // may stretch hundreds of metres above and below the drivable terrain.
                from[1] = ground.point[1] + 0.6;
                to[1] = from[1];
                tested += 1;
                let Some(raw) = world.ray_cast(from, to, &options) else { continue };
                if raw.kind != HitKind::Barrier || accept_wall(&raw) {
                    continue;
                }
                let Some(filtered) = world.ray_cast_filtered(from, to, &options, accept_wall) else { continue };
                assert!(accept_wall(&filtered));
                assert!(filtered.t >= raw.t, "filtering cannot create an earlier crossing");
                assert_eq!(world.ray_cast_filtered(from, to, &options, |_| true), Some(raw));
                witnessed += 1;
                eprintln!(
                    "road-height probe {from:?} -> {to:?}: rejected {}/{} at {:.5}, accepted {}/{} at {:.5}",
                    raw.section, raw.instance, raw.t, filtered.section, filtered.instance, filtered.t
                );
                if witnessed >= 4 || tested >= 4000 {
                    break 'packs;
                }
            }
        }
    }
    eprintln!("{witnessed} road-height backfaces hid eligible contacts among {tested} tested probes");
    assert!(witnessed >= 4, "need actual installed geometry exercising candidate rejection");
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn shallow_installed_faces_seen_from_below_remain_ground_surfaces() {
    let packs = read_collision_packs(&file("TRACKS/STREAML2RA.BUN")).unwrap();
    let mut checked = 0;
    for pack in packs {
        let mut world = CollisionWorld::new(None);
        world.insert(pack.clone());
        for (index, inst) in pack.instances.iter().enumerate() {
            if inst.group != 0 {
                continue;
            }
            for strip in &pack.article_of(index).unwrap().strips {
                let tri = strip.triangle(0);
                let middle = inst.to_world([0, 1, 2].map(|k| tri.pts.iter().map(|p| p[k]).sum::<f32>() / 3.0));
                let from = sub(middle, [0.0, 0.02, 0.0]);
                let to = add(middle, [0.0, 0.02, 0.0]);
                let options =
                    RayOptions { barriers: false, exclude: u32::from(GROUP_EXCLUSION), ..RayOptions::default() };
                let Some(raw) = world.ray_cast(from, to, &options) else { continue };
                if raw.normal[1] >= -0.7 {
                    continue;
                }
                assert!(!accept_wall(&raw), "origin-facing downward normals still identify shallow faces");
                if let Some(accepted) = world.ray_cast_filtered(from, to, &options, accept_wall) {
                    assert!(accept_wall(&accepted), "a later steep surface may still be a valid wall");
                }
                checked += 1;
                if checked >= 32 {
                    eprintln!("{checked} installed shallow faces have downward raw normals and are rejected as walls");
                    return;
                }
            }
        }
    }
    panic!("only {checked} installed shallow faces checked");
}

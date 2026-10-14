//! Bounded installed-world query smoke; original instruction fixtures live in tests.rs.

use super::{GroundFaceCache, GroundReferenceCache, GroundReferenceSettings, PointQuerySettings, RegionSettings};
use crate::{CollisionWorld, Grid, read_collision_packs};

fn game_file(path: &str) -> Vec<u8> {
    let dir = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR for installed ground queries");
    let file = std::path::Path::new(&dir).join(path);
    let bytes = std::fs::read(&file).unwrap_or_else(|error| panic!("{}: {error}", file.display()));
    ea_compress::unwrap(&bytes).expect("game compression").into_owned()
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_cached_ground_queries() {
    let packs = read_collision_packs(&game_file("TRACKS/STREAML2RA.BUN")).expect("packs");
    let grid = Grid::read(&game_file("TRACKS/L2RA.BUN")).expect("grid parse").expect("grid");
    let mut points = Vec::new();
    'packs: for pack in &packs {
        for (index, instance) in pack.instances.iter().enumerate() {
            if instance.group != 0 || instance.flags != 0 {
                continue;
            }
            let article = pack.article_of(index).expect("resident article");
            for strip in &article.strips {
                let Some(triangle) = strip.triangles().next() else { continue };
                if triangle.flags & 8 != 0 {
                    continue;
                }
                let pts = triangle.pts.map(|point| instance.to_world(point));
                let horizontal_area = (pts[1][0] - pts[0][0]) * (pts[2][2] - pts[0][2])
                    - (pts[1][2] - pts[0][2]) * (pts[2][0] - pts[0][0]);
                if horizontal_area.abs() < 1e-4 {
                    continue;
                }
                let point = [
                    (pts[0][0] + pts[1][0] + pts[2][0]) / 3.0,
                    (pts[0][1] + pts[1][1] + pts[2][1]) / 3.0 + 2.0,
                    (pts[0][2] + pts[1][2] + pts[2][2]) / 3.0,
                ];
                points.push(point);
                if points.len() == 64 {
                    break 'packs;
                }
            }
        }
    }
    assert_eq!(points.len(), 64, "installed fixture must exercise nonzero geometry");
    let mut world = CollisionWorld::new(Some(grid));
    for pack in packs {
        world.insert(pack);
    }
    let settings = GroundReferenceSettings {
        region: RegionSettings {
            initial_expansion: 1.1,
            motion_threshold: 5.0,
            prediction: 2.5,
            padding: 0.1,
            maximum_radius: 25.0,
        },
        vertical_tolerance: 0.025,
        minimum_vertical_separation: -1.0,
        maximum_normal_y: 0.9999,
        edge_length_epsilon: 1e-6,
        continuation_mask: 4,
        instance_exclusion: 0,
        maximum_cell_span: 20,
        reuse_face: true,
        preserve_face: true,
    };
    let mut valid = 0;
    let mut global_valid = 0;
    let point_settings = PointQuerySettings {
        cache: settings,
        minimum_face_offset: 0.5,
        unknown_winding_height: 1.0,
        downward_distance: 750.0,
        upright_face_exclusion: 8,
    };
    for point in points {
        let mut cache = GroundReferenceCache::default();
        let plane = cache.update(&world, point, 3.0, &settings, |_| false);
        assert!(plane.normal.into_iter().all(f32::is_finite));
        assert!(plane.separation.is_finite());
        if plane.valid {
            valid += 1;
            assert!(plane.normal[1] >= 0.0 && plane.normal[1] <= settings.maximum_normal_y);
        }
        let mut cache = GroundFaceCache::default();
        let plane = cache.update_global(&world, point, &point_settings, |_| false);
        if plane.valid {
            global_valid += 1;
        }
        assert!(plane.normal.into_iter().all(f32::is_finite) && plane.separation.is_finite());
    }
    assert!(valid >= 48, "only {valid}/64 installed ground queries selected faces");
    assert!(global_valid >= 48, "only {global_valid}/64 original global ground queries selected faces");
}

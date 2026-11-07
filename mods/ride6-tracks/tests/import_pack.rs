//! A synthetic pack through the whole import: the files come out, parse with the game's own readers and manifests, and the
//! collision answers a ray with the surface of the material.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use blackbox_collision::{CollisionWorld, RayOptions};
use blackbox_modding::content::{Collision, parse_scenery, parse_world};
use ride6_tracks::import::import;
use ride6_tracks::pack::{Group, write_groups};

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ride6-tracks-import-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn png(path: &Path, size: u32, colour: [u8; 4]) {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, size, size);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let rows: Vec<u8> = (0..size * size).flat_map(|_| colour).collect();
    encoder.write_header().unwrap().write_image_data(&rows).unwrap();
    std::fs::write(path, out).unwrap();
}

/// A flat square of `side` metres from `(x0, y0)` at height `z`, in render space.
fn square(material: usize, x0: f32, y0: f32, side: f32, z: f32) -> Group {
    let corners = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    Group {
        material,
        collide: true,
        positions: corners.iter().map(|c| [x0 + c[0] * side, y0 + c[1] * side, z]).collect(),
        uvs: corners.to_vec(),
        normals: vec![[0, 0, 127]; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

fn write_pack(dir: &Path) {
    std::fs::create_dir_all(dir.join("tex")).unwrap();
    png(&dir.join("tex/t_grass.png"), 8, [40, 160, 40, 255]);
    let json = r#"{
 "format": 1, "track": "Test Track", "triangles": 4,
 "materials": [
  {"name": "tarmac", "texture": null, "color": [60, 60, 70], "tile_m": 10.0, "uv": "planar", "surface": "asphalt"},
  {"name": "grass", "texture": "t_grass", "color": [0, 200, 0], "tile_m": 10.0, "uv": "planar", "surface": "grass"}
 ],
 "lap": {"grid": [[5.0, 5.0, 1.0, 90.0]]}
}"#;
    std::fs::write(dir.join("pack.json"), json).unwrap();
    // Two squares 1,000 m apart: two cells.
    let groups = vec![square(0, 0.0, 0.0, 20.0, 1.0), square(1, 1000.0, 0.0, 20.0, 2.0)];
    std::fs::write(dir.join("ground.r6m"), write_groups(&groups)).unwrap();
}

#[test]
fn a_pack_becomes_cells_a_world_manifest_and_ground_that_answers_rays() {
    let (pack, out) = (temp("pack"), temp("out"));
    write_pack(&pack);
    let report = import(&pack, &out, 64, &AtomicBool::new(false)).unwrap();
    assert_eq!((report.world.as_str(), report.cells, report.triangles), ("testtrack", 2, 4));
    assert_eq!((report.collision_triangles, report.collision_skipped), (4, 0));
    assert_eq!(report.pictures, 1);
    assert!(report.missing_pictures.is_empty());

    let world = parse_world("testtrack", &std::fs::read_to_string(out.join("worlds/testtrack.toml")).unwrap()).unwrap();
    assert_eq!(world.name, "Test Track (RIDE 6)");
    assert_eq!((world.start, world.heading), ([5.0, 5.0, 1.0], 90.0));

    let mut world_ground = CollisionWorld::new(None);
    let mut surfaces = Vec::new();
    for key in ["g000", "g001"] {
        let manifest = std::fs::read_to_string(out.join(format!("worlds/testtrack/{key}.toml"))).unwrap();
        let pack = parse_scenery(key, &manifest).unwrap();
        assert_eq!(pack.places[0].collision, Collision::Static);
        let bytes = std::fs::read(out.join("files").join(&pack.file)).unwrap();
        let solids = blackbox_solid::read_solids(&bytes).unwrap();
        assert!(pack.places.iter().all(|place| solids.iter().any(|s| s.name == place.model)));
        assert_eq!(blackbox_tpk::read_texture_packs(&bytes).unwrap()[0].textures.len(), 1);
        for collision in blackbox_collision::read_collision_packs(&bytes).unwrap() {
            assert_eq!(collision.section, u32::from(pack.section));
            world_ground.insert_ungridded(collision);
        }
        surfaces.push(pack.section);
    }
    assert_eq!(surfaces, vec![2700, 2701]);
    // Render (x, y, z) is physics (-y, z, x): the first square's point (6, 7) is physics (x -7, z 6).
    let hit = |x: f32, y: f32| {
        world_ground.ray_cast([-y, 50.0, x], [-y, -50.0, x], &RayOptions::default()).expect("ground under the ray")
    };
    let road = hit(6.0, 7.0);
    assert!((road.point[1] - 1.0).abs() < 0.02);
    assert_eq!(road.surface_hash, 0x19DB_2F1E);
    let verge = hit(1006.0, 7.0);
    assert!((verge.point[1] - 2.0).abs() < 0.02);
    assert_eq!(verge.surface_hash, 0x772F_B736);
    let _ = std::fs::remove_dir_all(&pack);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn a_missing_picture_is_reported_and_its_material_is_drawn_flat() {
    let (pack, out) = (temp("flat-pack"), temp("flat-out"));
    write_pack(&pack);
    std::fs::remove_file(pack.join("tex/t_grass.png")).unwrap();
    let report = import(&pack, &out, 64, &AtomicBool::new(false)).unwrap();
    assert_eq!(report.pictures, 0);
    assert_eq!(report.missing_pictures.len(), 1);
    assert!(out.join("files/WORLDS/TESTTRACK/G001.BUN").is_file());
    let _ = std::fs::remove_dir_all(&pack);
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn a_cancelled_import_stops_with_an_error() {
    let (pack, out) = (temp("cancel-pack"), temp("cancel-out"));
    write_pack(&pack);
    let error = import(&pack, &out, 64, &AtomicBool::new(true)).unwrap_err();
    assert!(error.to_string().contains("cancelled"));
    let _ = std::fs::remove_dir_all(&pack);
    let _ = std::fs::remove_dir_all(&out);
}

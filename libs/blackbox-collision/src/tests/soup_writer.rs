//! The soup writer: what it writes reads back as the same ground, and a ray cast finds it.

use blackbox_chunk::ChunkWriter;

use crate::{
    CollisionPack, CollisionWorld, RayOptions, SoupOptions, SoupTriangle, read_collision_packs, write_soup_pack,
};

const ASPHALT: u32 = 0x19DB_2F1E;
const GRASS: u32 = 0x772F_B736;

/// A flat square of `side` metres at height `y`, from `(x0, z0)`, as two triangles with the given surface.
fn square(x0: f32, z0: f32, side: f32, y: f32, surface: u32) -> [SoupTriangle; 2] {
    let p = |dx: f32, dz: f32| [x0 + dx * side, y, z0 + dz * side];
    [
        SoupTriangle { points: [p(0.0, 0.0), p(0.0, 1.0), p(1.0, 1.0)], surface },
        SoupTriangle { points: [p(0.0, 0.0), p(1.0, 1.0), p(1.0, 0.0)], surface },
    ]
}

fn written(section: u32, triangles: &[SoupTriangle], options: &SoupOptions) -> (Vec<u8>, crate::SoupReport) {
    let mut out = ChunkWriter::new();
    // A leading chunk, so the pack's alignment fill is exercised.
    out.leaf(0x1234, &[1, 2, 3]);
    let report = write_soup_pack(&mut out, section, triangles, options).unwrap();
    (out.finish().unwrap(), report)
}

#[test]
fn a_soup_reads_back_with_its_triangles_and_surfaces() {
    let mut triangles = Vec::new();
    triangles.extend(square(0.0, 0.0, 10.0, 2.0, ASPHALT));
    triangles.extend(square(10.0, 0.0, 10.0, 2.5, GRASS));
    let (bytes, report) = written(2700, &triangles, &SoupOptions::default());
    assert_eq!((report.triangles, report.skipped), (4, 0));
    let packs = read_collision_packs(&bytes).unwrap();
    assert_eq!(packs.len(), 1);
    let pack: &CollisionPack = &packs[0];
    assert_eq!(pack.section, 2700);
    assert_eq!(pack.instances.len(), report.instances);
    let read: usize = pack.articles.iter().map(|a| a.strips.iter().map(|s| s.triangle_count()).sum::<usize>()).sum();
    assert_eq!(read, 4);
    let hashes: std::collections::BTreeSet<u32> = pack.articles.iter().flat_map(|a| a.surfaces.clone()).collect();
    assert_eq!(hashes, [ASPHALT, GRASS].into_iter().collect());
}

#[test]
fn a_ray_down_finds_the_height_and_the_surface() {
    let mut triangles = Vec::new();
    triangles.extend(square(0.0, 0.0, 10.0, 2.0, ASPHALT));
    triangles.extend(square(10.0, 0.0, 10.0, 2.5, GRASS));
    let (bytes, _) = written(2700, &triangles, &SoupOptions::default());
    let mut world = CollisionWorld::new(None);
    world.insert_ungridded(read_collision_packs(&bytes).unwrap().remove(0));
    let down = |x: f32, z: f32| world.ray_cast([x, 20.0, z], [x, -20.0, z], &RayOptions::default());
    let road = down(2.0, 7.0).expect("the road is hit");
    assert!((road.point[1] - 2.0).abs() < 0.02, "height {}", road.point[1]);
    assert_eq!(road.surface_hash, ASPHALT);
    let verge = down(12.0, 8.0).expect("the grass is hit");
    assert!((verge.point[1] - 2.5).abs() < 0.02);
    assert_eq!(verge.surface_hash, GRASS);
    assert!(down(40.0, 7.0).is_none(), "nothing beyond the soup");
}

#[test]
fn a_large_soup_is_cut_into_small_instances() {
    let mut triangles = Vec::new();
    for i in 0..40 {
        for j in 0..40 {
            triangles.extend(square(i as f32 * 5.0, j as f32 * 5.0, 5.0, 0.0, ASPHALT));
        }
    }
    let options = SoupOptions { max_triangles: 300, max_extent: 40.0 };
    let (bytes, report) = written(2701, &triangles, &options);
    assert_eq!(report.triangles, 3200);
    assert!(report.instances >= 3200 / 300, "{} instances", report.instances);
    let pack = read_collision_packs(&bytes).unwrap().remove(0);
    for (inst, article) in pack.instances.iter().zip(&pack.articles) {
        let count: usize = article.strips.iter().map(|s| s.triangle_count()).sum();
        assert!(count <= 300);
        assert!(inst.half_width <= 20.2 && inst.half_length <= 20.2, "box {} x {}", inst.half_width, inst.half_length);
    }
}

#[test]
fn a_triangle_too_wide_for_a_strip_is_left_out_and_counted() {
    let wide = SoupTriangle { points: [[0.0, 0.0, 0.0], [900.0, 0.0, 0.0], [0.0, 0.0, 900.0]], surface: ASPHALT };
    let (_, report) = written(2702, &[wide], &SoupOptions::default());
    assert_eq!((report.triangles, report.skipped), (0, 1));
}

#[test]
fn too_many_triangles_per_instance_is_refused() {
    let mut out = ChunkWriter::new();
    let options = SoupOptions { max_triangles: 5000, max_extent: 10.0 };
    assert!(write_soup_pack(&mut out, 1, &[], &options).is_err());
}

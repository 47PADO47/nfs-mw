//! Against a real NFS: Most Wanted install (set `NFSMW_GAME_DIR`). Expected numbers are the ones
//! in `docs/formats/collision.md`.

use std::collections::HashSet;

use blackbox_attrib::{Database, vlt_hash};

use crate::{
    BoundsSet, CollisionPack, CollisionWorld, Grid, HitKind, RayOptions, Shape, read_bounds_sets, read_collision_packs,
};

fn game_file(path: &str) -> Option<Vec<u8>> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    let path = std::path::Path::new(&dir).join(path);
    Some(std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

fn packs() -> Option<Vec<CollisionPack>> {
    Some(read_collision_packs(&game_file("TRACKS/STREAML2RA.BUN")?).expect("collision packs"))
}

const SIMSURFACE: u32 = 0xFB11_1FEF;

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_collision_packs() {
    let Some(packs) = packs() else { return };
    assert_eq!(packs.len(), 390);
    let sections: HashSet<_> = packs.iter().map(|p| p.section).collect();
    assert_eq!(sections.len(), 390);
    assert_eq!((packs.iter().map(|p| p.section).min(), packs.iter().map(|p| p.section).max()), (Some(101), Some(2030)));

    let articles = packs.iter().flat_map(|p| &p.articles);
    assert_eq!(packs.iter().map(|p| p.instances.len()).sum::<usize>(), 7544);
    assert_eq!(articles.clone().count(), 7544);
    assert_eq!(packs.iter().map(|p| p.objects.len()).sum::<usize>(), 0);
    assert_eq!(articles.clone().map(|a| a.strips.len()).sum::<usize>(), 38_379);
    assert_eq!(articles.clone().flat_map(|a| &a.strips).map(|s| s.triangle_count()).sum::<usize>(), 243_429);
    assert_eq!(articles.clone().flat_map(|a| &a.strips).map(|s| s.verts.len()).sum::<usize>(), 320_187);
    assert_eq!(articles.clone().map(|a| a.barriers.len()).sum::<usize>(), 45_815);

    // Every instance is upright and axis aligned in this install; its article index is its own.
    for p in &packs {
        for (i, inst) in p.instances.iter().enumerate() {
            assert_eq!(usize::from(inst.article), i);
            assert_eq!((inst.row_x, inst.row_z, inst.flags), ([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], 0));
            assert!(inst.radius + 1e-3 >= inst.half_width.hypot(inst.half_length));
        }
    }

    // Every surface hash is a collection of the `simsurface` class (hash 0 = none).
    let db = Database::open(&game_file("GLOBAL/attributes.bin").unwrap()).expect("attributes.bin");
    let hashes: HashSet<u32> = articles.flat_map(|a| a.surfaces.iter().copied()).collect();
    assert_eq!(hashes.len(), 20);
    let mut unknown: Vec<u32> =
        hashes.iter().copied().filter(|&h| h != 0 && db.collection_by_key(SIMSURFACE, h).is_none()).collect();
    unknown.sort_unstable();
    // Two of them have no collection (the game then uses `unknown`).
    let mut unknown: Vec<u32> =
        hashes.iter().copied().filter(|&h| h != 0 && db.collection_by_key(SIMSURFACE, h).is_none()).collect();
    unknown.sort_unstable();
    assert_eq!(unknown, [0x3319_9393, vlt_hash("sidewalk")]);
    for name in ["asphalt", "concrete", "grass", "cobble", "wood", "gravel", "metal", "dirt", "sand"] {
        assert!(hashes.contains(&vlt_hash(name)), "{name}");
    }
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_grid_and_ray_casts() {
    let Some(packs) = packs() else { return };
    let grid = Grid::read(&game_file("TRACKS/L2RA.BUN").unwrap()).expect("grid").expect("a grid chunk");
    assert_eq!((grid.rows, grid.cols, grid.edge, grid.min), (103, 92, 64.0, [-4800.0, 0.0, -1664.0]));

    // The grid lists every instance exactly through references that resolve.
    let mut listed = HashSet::new();
    let mut nodes = 0;
    for index in 0..(grid.rows * grid.cols) as usize {
        let Some(n) = grid.node(index) else { continue };
        nodes += 1;
        assert_eq!(usize::from(n.index), index);
        listed.extend(n.instances.iter().copied());
    }
    assert_eq!(nodes, 2779);
    assert_eq!(listed.len(), 7544);
    let by_section: std::collections::HashMap<_, _> = packs.iter().map(|p| (p.section, p)).collect();
    assert!(listed.iter().all(|r| by_section.get(&r.section()).is_some_and(|p| r.index() < p.instances.len())));

    // Cast straight down onto the middle of the first triangle of every strip-bearing article of
    // the first sections: something must be hit within half a metre of that triangle.
    let near: Vec<&CollisionPack> = packs.iter().filter(|p| p.section < 130).collect();
    let mut world = CollisionWorld::new(Some(grid));
    for p in &near {
        world.insert((*p).clone());
    }
    let mut tested = 0;
    for p in near {
        for (i, inst) in p.instances.iter().enumerate() {
            let Some(strip) = p.article_of(i).and_then(|a| a.strips.first()) else { continue };
            let tri = strip.triangle(0);
            let c = [0, 1, 2].map(|k| (tri.pts[0][k] + tri.pts[1][k] + tri.pts[2][k]) / 3.0);
            let w = inst.to_world(c);
            let hit = world.ray_cast([w[0], w[1] + 0.5, w[2]], [w[0], w[1] - 0.5, w[2]], &RayOptions::default());
            let hit = hit.unwrap_or_else(|| panic!("no hit under instance {}/{i}", p.section));
            assert!((hit.point[0] - w[0]).abs() < 1e-3 && (hit.point[2] - w[2]).abs() < 1e-3);
            assert!((hit.point[1] - w[1]).abs() <= 0.5 + 1e-3);
            assert_eq!(hit.kind, HitKind::Face);
            tested += 1;
        }
    }
    assert!(tested > 20, "only {tested} strips tried");
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn real_install_bounds() {
    let Some(file) = game_file("GLOBAL/GlobalB.lzc") else { return };
    let cars = read_bounds_sets(&file).expect("car bounds");
    assert_eq!(cars.len(), 86);
    assert_eq!(cars.iter().map(|s| s.nodes.len()).sum::<usize>(), 1066);
    assert_eq!(cars.iter().map(|s| s.point_clouds.len()).sum::<usize>(), 97);
    assert!(cars.iter().all(BoundsSet::is_tree));

    // Car bounds are keyed by the AttribSys hash of the car type name.
    let m3 = crate::find_bounds(&cars, vlt_hash("BMWM3GTR")).expect("BMWM3GTR");
    assert_eq!(m3.nodes.len(), 10);
    assert_eq!(m3.root().half_dimensions, [0.938, 0.621, 2.271]);
    assert_eq!((m3.root().shape(), m3.children(m3.root()).len()), (Shape::Box, 9));
    assert_eq!(m3.point_clouds.iter().map(Vec::len).collect::<Vec<_>>(), [16]);
    let spheres = m3.nodes.iter().filter(|n| n.shape() == Shape::Sphere).count();
    assert_eq!(spheres, 6);
    for missing in ["BMWM3", "TRUENO", "LEVIN", "TRUENOCP", "TRUENOID"] {
        assert!(crate::find_bounds(&cars, vlt_hash(missing)).is_none(), "{missing}");
    }

    let props = read_bounds_sets(&game_file("TRACKS/L2RA.BUN").unwrap()).expect("prop bounds");
    assert_eq!(props.len(), 405);
    assert_eq!(props.iter().map(|s| s.nodes.len()).sum::<usize>(), 2141);
    assert_eq!(props.iter().map(|s| s.point_clouds.len()).sum::<usize>(), 332);
    assert!(props.iter().all(BoundsSet::is_tree));
}

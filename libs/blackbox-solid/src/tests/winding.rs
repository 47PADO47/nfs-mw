//! Against a real NFS: Most Wanted install (set `NFSMW_GAME_DIR`): which way the triangles of closed models are wound.
//! A renderer that culls back faces keeps the triangles whose corners run counter-clockwise (right-hand rule) around
//! the normal facing the camera; this checks that the car models agree with their stored vertex normals. Numbers are
//! in `docs/specs/backface-culling.md`.

use crate::{Solid, read_solids_lossy};

/// Triangles whose right-hand-rule normal points the same way as the sum of their vertex normals, and the others.
/// Degenerate triangles are not counted.
fn winding(solid: &Solid) -> (usize, usize) {
    let mut counts = (0, 0);
    for group in &solid.groups {
        let first = group.first_index as usize;
        let indices = &solid.indices[first..first + group.num_indices as usize];
        for &[ia, ib, ic] in indices.as_chunks::<3>().0 {
            let vertex = |i: u16| solid.vertices.get(group.base_vertex as usize + usize::from(i));
            let (Some(a), Some(b), Some(c)) = (vertex(ia), vertex(ib), vertex(ic)) else { continue };
            let sub = |p: [f32; 3], q: [f32; 3]| [p[0] - q[0], p[1] - q[1], p[2] - q[2]];
            let (e1, e2) = (sub(b.position, a.position), sub(c.position, a.position));
            let n = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
            let along: f32 = (0..3).map(|axis| n[axis] * (a.normal[axis] + b.normal[axis] + c.normal[axis])).sum();
            if along.abs() < 1e-12 {
                continue;
            }
            match along > 0.0 {
                true => counts.0 += 1,
                false => counts.1 += 1,
            }
        }
    }
    counts
}

#[test]
#[ignore = "reads the game install (NFSMW_GAME_DIR)"]
fn car_models_are_wound_counter_clockwise_around_their_normals() {
    let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let cars = std::path::Path::new(&dir).join("CARS");
    let (mut agree, mut disagree, mut files) = (0usize, 0usize, 0usize);
    for entry in std::fs::read_dir(&cars).unwrap().flatten() {
        let Ok(data) = std::fs::read(entry.path().join("GEOMETRY.BIN")) else { continue };
        let Ok((solids, _)) = read_solids_lossy(&data) else { continue };
        files += 1;
        for (a, d) in solids.iter().map(winding) {
            agree += a;
            disagree += d;
        }
    }
    eprintln!("{files} car geometry files: {agree} triangles agree with their normals, {disagree} do not");
    assert!(files > 80, "the install has about 90 car folders");
    assert!(disagree * 1000 < agree, "fewer than 0.1% of car triangles are wound against their normals");
}

//! The far ground: the whole track's ground with its vertices clustered on a coarse grid, as one cell that is always loaded.
//! The near cells stream in by the camera's distance (`docs/specs/second-world.md`); beyond them the far ground is what is
//! drawn, so the horizon is not black. It lies a little below the real ground, so wherever a near cell is loaded that covers it.

use std::collections::HashMap;

use crate::pack::{Group, Material, Pack};

/// How far below the real ground the far ground lies, metres: the clusters average heights, so on a slope the coarse surface can
/// stand above the fine one, and this keeps it from showing through.
pub const BIAS: f32 = 2.0;
/// Groups with fewer triangles than this are left out of the far ground (kerbs, painted lines: a few metres wide, no use afar).
const MIN_GROUP_TRIANGLES: usize = 3000;
/// The side of the sea quad, metres.
const SEA_SIDE: f32 = 80_000.0;
/// The colour of the plane that stands in for a sea when the track has none, under the lowest ground.
const PLAIN_COLOUR: [u8; 3] = [64, 84, 52];

/// One cluster of vertices: the sums of what its members have.
#[derive(Default)]
struct Cluster {
    position: [f32; 3],
    uv: [f32; 2],
    normal: [f32; 3],
    count: f32,
}

/// The pack's ground with `grid` metre clusters, below the real ground by [`BIAS`], with a sea (or plain) quad added. Nothing in it is
/// collision.
pub fn far_pack(pack: &Pack, grid: f32) -> Pack {
    let mut groups: Vec<Group> = pack
        .groups
        .iter()
        .filter(|group| group.indices.len() / 3 >= MIN_GROUP_TRIANGLES)
        .map(|group| cluster(group, grid))
        .filter(|group| !group.indices.is_empty())
        .collect();
    let mut materials: Vec<Material> = pack.materials.clone();
    let low = pack.groups.iter().flat_map(|g| g.positions.iter().map(|p| p[2])).fold(f32::INFINITY, f32::min);
    let (z, color) = match pack.sea {
        Some(sea) => (sea.z, sea.color),
        None => (if low.is_finite() { low - 3.0 } else { 0.0 }, PLAIN_COLOUR),
    };
    materials.push(Material { name: "sea".into(), texture: None, color, surface: "dirt".into() });
    groups.push(quad(materials.len() - 1, z - BIAS));
    Pack { track: pack.track.clone(), materials, groups, lap: Default::default(), sea: None }
}

/// A square of `SEA_SIDE` metres at height `z`, facing up.
fn quad(material: usize, z: f32) -> Group {
    let h = SEA_SIDE / 2.0;
    Group {
        material,
        collide: false,
        positions: vec![[-h, -h, z], [h, -h, z], [h, h, z], [-h, h, z]],
        uvs: vec![[0.0, 0.0]; 4],
        normals: vec![[0, 0, 127]; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

/// `group` with its vertices merged by the `grid` metre square (x and y) they are in; a triangle whose corners fall in fewer than
/// three squares is dropped.
fn cluster(group: &Group, grid: f32) -> Group {
    let mut ids: HashMap<(i32, i32), u32> = HashMap::new();
    let mut clusters: Vec<Cluster> = Vec::new();
    let mapped: Vec<u32> = group
        .positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let key = ((p[0] / grid).floor() as i32, (p[1] / grid).floor() as i32);
            let id = *ids.entry(key).or_insert_with(|| {
                clusters.push(Cluster::default());
                clusters.len() as u32 - 1
            });
            let c = &mut clusters[id as usize];
            for k in 0..3 {
                c.position[k] += p[k];
                c.normal[k] += f32::from(group.normals[i][k]);
            }
            c.uv[0] += group.uvs[i][0];
            c.uv[1] += group.uvs[i][1];
            c.count += 1.0;
            id
        })
        .collect();
    let indices: Vec<u32> = group
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .filter(|t| {
            let (a, b, c) = (mapped[t[0] as usize], mapped[t[1] as usize], mapped[t[2] as usize]);
            a != b && b != c && a != c
        })
        .flat_map(|t| t.iter().map(|&i| mapped[i as usize]))
        .collect();
    Group {
        material: group.material,
        collide: false,
        positions: clusters
            .iter()
            .map(|c| [c.position[0] / c.count, c.position[1] / c.count, c.position[2] / c.count - BIAS])
            .collect(),
        uvs: clusters.iter().map(|c| [c.uv[0] / c.count, c.uv[1] / c.count]).collect(),
        normals: clusters
            .iter()
            .map(|c| {
                let length = (c.normal[0].powi(2) + c.normal[1].powi(2) + c.normal[2].powi(2)).sqrt().max(1e-6);
                c.normal.map(|n| (n / length * 127.0).round() as i8)
            })
            .collect(),
        indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::{Lap, Sea};

    /// A flat field of `n` by `n` unit squares from the origin at height 5, as one group.
    fn field(n: usize) -> Group {
        let mut positions = Vec::new();
        for y in 0..=n {
            for x in 0..=n {
                positions.push([x as f32, y as f32, 5.0]);
            }
        }
        let mut indices = Vec::new();
        for y in 0..n {
            for x in 0..n {
                let a = (y * (n + 1) + x) as u32;
                let (b, c) = (a + 1, a + (n + 1) as u32);
                indices.extend([a, b, c + 1, a, c + 1, c]);
            }
        }
        let count = positions.len();
        Group {
            material: 0,
            collide: true,
            positions,
            uvs: vec![[0.5, 0.5]; count],
            normals: vec![[0, 0, 127]; count],
            indices,
        }
    }

    fn pack(sea: Option<Sea>) -> Pack {
        let material = Material { name: "m".into(), texture: None, color: [1, 2, 3], surface: "grass".into() };
        Pack { track: "T".into(), materials: vec![material], groups: vec![field(80)], lap: Lap::default(), sea }
    }

    #[test]
    fn the_far_ground_has_far_fewer_triangles_lies_below_and_is_no_collision() {
        let far = far_pack(&pack(None), 8.0);
        let ground = &far.groups[0];
        assert!(ground.indices.len() / 3 < 80 * 80 * 2 / 10, "{} triangles", ground.indices.len() / 3);
        assert!(far.groups.iter().all(|g| !g.collide));
        assert!(ground.positions.iter().all(|p| (p[2] - (5.0 - BIAS)).abs() < 1e-4));
        assert!(ground.indices.iter().all(|&i| (i as usize) < ground.positions.len()));
    }

    #[test]
    fn a_sea_or_a_plain_is_added_under_the_ground() {
        let with_sea = far_pack(&pack(Some(Sea { z: 1.0, color: [10, 20, 90] })), 8.0);
        let quad = with_sea.groups.last().unwrap();
        assert_eq!(with_sea.materials.last().unwrap().color, [10, 20, 90]);
        assert!((quad.positions[0][2] - (1.0 - BIAS)).abs() < 1e-4);
        let plain = far_pack(&pack(None), 8.0);
        assert!(plain.groups.last().unwrap().positions[0][2] < 5.0 - BIAS, "under the lowest ground");
    }

    #[test]
    fn small_groups_are_left_out() {
        let mut small = pack(None);
        small.groups = vec![field(10)];
        assert_eq!(far_pack(&small, 8.0).groups.len(), 1, "only the sea quad");
    }
}

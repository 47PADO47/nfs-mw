//! Enhanced emitter profiles for recognized tunnel mesh families. No original physical-light values are claimed.
use crate::scenes::world::resident::Placed;
use blackbox_gfx::MeshHandle;
use glam::Vec3;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub(in crate::scenes::world) struct Fixture {
    pub position: Vec3,
    pub direction: Vec3,
    pub street: bool,
    pub lens_half: glam::Vec2,
    pub glow_position: Vec3,
    pub lantern_half_height: f32,
    /// The uploaded mesh draws its own emissive glass (`lantern.rs`), so no glow sprite stands in for it.
    pub glass: bool,
}
impl Fixture {
    pub(in crate::scenes::world) fn read(solid: &blackbox_solid::Solid) -> Vec<Self> {
        let name = solid.name.to_ascii_uppercase();
        // Only catalogued models with independently inspected mesh groups are eligible.
        let lens_indices = super::catalog::lens_indices(&name);
        let street = lens_indices != 0;
        let lantern = name.starts_with("XO_IRONLAMP") || name.starts_with("XO_PARKLAMPA_");
        let round = name == "XO_TUNNELLIGHTA_1B_LL_00";
        let strip = name.starts_with("XO_TUNNEL_LIGHT_LONG_") || name.starts_with("XO_TUNNEL_LIGHT_RUNNING_");
        let combined = name.contains("_DEINST");
        if !round && !strip && !street {
            return Vec::new();
        }
        // The catalogued group must be the only candidate; an ambiguous layout is not guessed.
        let mut groups = solid.groups.iter().filter(|g| match street {
            true => {
                g.num_indices == lens_indices
                    && super::catalog::lens_texture(&name).is_some_and(|t| g.diffuse_texture(solid) == Some(t))
            }
            false => g.effect_id == 3,
        });
        let (Some(group), None) = (groups.next(), groups.next()) else {
            return Vec::new();
        };
        let Some(indices) = group_indices(solid, group) else {
            return Vec::new();
        };
        let Ok(base) = usize::try_from(group.base_vertex) else {
            return Vec::new();
        };
        let vertex = |index: u16| base.checked_add(usize::from(index)).and_then(|at| solid.vertices.get(at));
        // Each dual-head lens group spans both sides of the pole. Split triangles by local side,
        // not by index order, which is not an asset-format guarantee.
        let dual = name.starts_with("XO_LIGHTPOSTB_") || name.starts_with("XO_STREETLIGHTI_");
        let mut halves = [Vec::new(), Vec::new()];
        if dual {
            let (triangles, remainder) = indices.as_chunks::<3>();
            if !remainder.is_empty() {
                return Vec::new();
            }
            for triangle in triangles {
                let mut y = 0.0;
                for &index in triangle {
                    let Some(vertex) = vertex(index) else {
                        return Vec::new();
                    };
                    y += vertex.position[1];
                }
                halves[usize::from(y >= 0.0)].extend_from_slice(triangle);
            }
            if halves.iter().any(Vec::is_empty) {
                return Vec::new();
            }
        }
        // Installed combined strip meshes retain 66-index fixture blocks, including baked world placements.
        let chunk = match combined {
            true => 66,
            false => indices.len().max(1),
        };
        let chunks: Vec<&[u16]> = match dual {
            true => halves.iter().map(Vec::as_slice).collect(),
            false => indices.chunks(chunk).collect(),
        };
        chunks
            .into_iter()
            .filter_map(|indices| {
                let mut min = Vec3::splat(f32::INFINITY);
                let mut max = Vec3::splat(f32::NEG_INFINITY);
                let mut normal = Vec3::ZERO;
                for &index in indices {
                    let vertex = vertex(index)?;
                    let position = Vec3::from_array(vertex.position);
                    min = min.min(position);
                    max = max.max(position);
                    normal += Vec3::from_array(vertex.normal);
                }
                // Baked strip vertex normals are not consistently rotated with the fixture placements.
                let direction = match (street, strip && combined) {
                    (true, _) | (_, true) => Vec3::NEG_Z,
                    _ => normal.try_normalize()?,
                };
                let position = (min + max) * 0.5 + direction * ((max - min).dot(direction.abs()) * 0.5 + 0.08);
                let lens_half = ((max - min).truncate() * 0.5).clamp(glam::Vec2::splat(0.08), glam::Vec2::splat(0.7));
                let glow_position = if lantern { (min + max) * 0.5 } else { position };
                let lantern_half_height = if lantern { (max.z - min.z) * 0.30 } else { 0.0 };
                Some(Self { position, direction, street, lens_half, glow_position, lantern_half_height, glass: false })
            })
            .collect()
    }
}

/// A placed object whose model carries fixtures: its place in its tile's list and the fixtures of its most
/// detailed fixture-bearing LOD.
#[derive(Clone, Debug)]
pub(in crate::scenes::world) struct FixturePlace {
    pub place: u32,
    pub fixtures: Arc<[Fixture]>,
    /// Bit `i` set: the mesh of LOD slot `i` draws emissive glass, so no glow sprite stands in while it is drawn.
    pub glass_lods: u8,
}

impl FixturePlace {
    /// The fixture-bearing objects of a tile's `placed` list; `fixtures` finds a mesh's fixtures.
    pub(in crate::scenes::world) fn index(
        placed: &[Placed],
        fixtures: impl Fn(MeshHandle) -> Option<Arc<[Fixture]>>,
    ) -> Vec<Self> {
        let mut index = Vec::new();
        for (place, p) in placed.iter().enumerate() {
            let Some(found) = p.lods.iter().flatten().find_map(|&mesh| fixtures(mesh)) else { continue };
            let Ok(place) = u32::try_from(place) else { break };
            let mut glass_lods = 0;
            for (slot, mesh) in p.lods.iter().enumerate() {
                if mesh.and_then(&fixtures).is_some_and(|f| f.iter().all(|f| f.glass)) {
                    glass_lods |= 1 << slot;
                }
            }
            index.push(Self { place, fixtures: found, glass_lods });
        }
        index
    }
}

/// The index range of `group`, or `None` when the asset's range overflows or lies outside the index list.
pub(super) fn group_indices<'a>(
    solid: &'a blackbox_solid::Solid,
    group: &blackbox_solid::ShadingGroup,
) -> Option<&'a [u16]> {
    let start = usize::try_from(group.first_index).ok()?;
    let end = start.checked_add(usize::try_from(group.num_indices).ok()?)?;
    solid.indices.get(start..end)
}

#[cfg(test)]
mod catalog_tests {
    use super::Fixture;
    use game_install::GameDir;
    use nfsmw_data::world::{WorldIndex, load_section};
    use std::collections::BTreeSet;

    #[test]
    #[ignore = "requires installed park and iron lamp meshes"]
    fn installed_park_and_iron_lamps_use_their_own_lens_geometry() {
        let expected = [
            "XO_PARKLAMPA_1B_00",
            "XO_PARKLAMPA_1C_00",
            "XO_PARKLAMPA_1Z_00",
            "XO_IRONLAMPHANGING_1B_00",
            "XO_IRONLAMPHANGING_1C_00",
            "XO_IRONLAMPHANGING_1Z_00",
            "XO_IRONLAMPSTANDING_1B_00",
            "XO_IRONLAMPSTANDING_1C_00",
            "XO_IRONLAMPSTANDING_1Z_00",
            "XO_IRONLAMPSTANDINGB_1B_00",
            "XO_IRONLAMPSTANDINGB_1C_00",
            "XO_IRONLAMPSTANDINGB_1Z_00",
        ];
        let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("game directory")).unwrap();
        let index = WorldIndex::open(&dir, "L2RA").unwrap();
        let mut stream = dir.open_file(&index.stream_file).unwrap();
        let mut found = BTreeSet::new();
        for (i, section) in index.sections.iter().enumerate() {
            let Ok(data) = load_section(&mut stream, i, section) else { continue };
            for solid in &data.solids {
                let name = solid.name.to_ascii_uppercase();
                if !expected.contains(&name.as_str()) || !found.insert(name.clone()) {
                    continue;
                }
                let fixtures = Fixture::read(solid);
                assert_eq!(fixtures.len(), 1, "{name} must have one light");
                let light = fixtures[0];
                assert!(light.street && light.position.is_finite() && light.direction == glam::Vec3::NEG_Z, "{name}");
                assert!(
                    (4.0..8.1).contains(&light.position.z),
                    "{name} light must be on the lamp head: {:?}",
                    light.position
                );
                assert!(
                    light.lantern_half_height > 0.0 && light.glow_position.z > light.position.z,
                    "{name} must illuminate the glass above the emitter"
                );
                if name.starts_with("XO_IRONLAMPHANGING") {
                    assert!(light.position.y < -2.8, "{name} light must be on the arm: {:?}", light.position);
                }
            }
        }
        assert_eq!(
            found.len(),
            expected.len(),
            "missing models: {:?}",
            expected.iter().filter(|name| !found.contains(**name)).collect::<Vec<_>>()
        );
    }
}

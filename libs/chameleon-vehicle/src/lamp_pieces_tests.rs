use super::*;
use crate::appearance::Appearance;
use chameleon_gfx::Vertex;

fn vertex(position: [f32; 3], uv: [f32; 2]) -> Vertex {
    Vertex { position, color: [255; 4], uv }
}

fn appearance(kind: SurfaceKind, technique: &str) -> Appearance {
    Appearance { kind, technique: technique.into(), diffuse_texture: None, diffuse_colour: None, alpha_test: false }
}

/// A mesh of the triangles `(corners, texel)`: every corner of a triangle reads the centre of texel `texel` of a 5x1 pattern.
fn mesh_of(kind: SurfaceKind, technique: &str, triangles: &[([[f32; 3]; 3], usize)]) -> PartMesh {
    let mut vertices = Vec::new();
    for (corners, texel) in triangles {
        let uv = [(*texel as f32 + 0.5) / 5.0, 0.5];
        vertices.extend(corners.iter().map(|&p| vertex(p, uv)));
    }
    let triangles = (0..triangles.len() as u32).map(|t| [3 * t, 3 * t + 1, 3 * t + 2]).collect();
    PartMesh { vertices, triangles, appearance: appearance(kind, technique) }
}

fn lens(triangles: &[([[f32; 3]; 3], usize)]) -> PartMesh {
    mesh_of(SurfaceKind::Glass, "VehicleNFS13_Refraction", triangles)
}

fn interior(triangles: &[([[f32; 3]; 3], usize)]) -> PartMesh {
    let technique = "VehicleNFS13_Body_Textured_NormalMap_EmissiveFourChannel_NoDamage_NoEffects";
    mesh_of(SurfaceKind::Light, technique, triangles)
}

fn strip(x: f32, z: f32) -> [[f32; 3]; 3] {
    [[x, 0.7, z], [x + 0.1, 0.7, z], [x, 0.7, z + 0.1]]
}

/// A 5x1 pattern: texel 0 lights the red channel (brake), 1 the green (head), 2 the blue (reverse), 3 the alpha (tail),
/// texel 4 is dark.
fn pattern() -> EmissiveMap {
    let texels: [[u8; 4]; 5] = [[255, 0, 0, 0], [0, 255, 0, 0], [0, 0, 255, 0], [0, 0, 0, 45], [0, 0, 0, 0]];
    let data: Vec<u8> = texels.iter().flatten().copied().collect();
    let texture = Texture {
        width: 5,
        height: 1,
        depth: 1,
        array_size: 1,
        mip_levels: 1,
        dxgi_format: 28,
        format: PixelFormat::Rgba8,
        data,
    };
    EmissiveMap::new(&texture).unwrap()
}

fn surfaces(pieces: &[PartMesh]) -> Vec<Option<LampSurface>> {
    pieces
        .iter()
        .map(|p| match p.appearance.kind {
            SurfaceKind::Lamp(surface) => Some(surface),
            _ => None,
        })
        .collect()
}

fn surface(end: LampEnd, side: LampSide, circuit: LampCircuit) -> Option<LampSurface> {
    Some(LampSurface { end, side, circuit })
}

#[test]
fn a_lens_splits_into_its_pieces_with_a_place_each() {
    // Front left, front right and a rear centre strip, apart from each other, and no pattern: each takes its end's circuit.
    let pieces = split(lens(&[(strip(0.5, 2.0), 0), (strip(-0.6, 2.0), 0), (strip(-0.05, -2.0), 0)]), None);
    assert_eq!(pieces.len(), 3);
    let found = surfaces(&pieces);
    assert!(found.contains(&surface(LampEnd::Front, LampSide::Left, LampCircuit::Head)));
    assert!(found.contains(&surface(LampEnd::Front, LampSide::Right, LampCircuit::Head)));
    assert!(found.contains(&surface(LampEnd::Rear, LampSide::Centre, LampCircuit::Brake)));
    assert!(pieces.iter().all(|p| p.triangles.len() == 1 && p.vertices.len() == 3));
}

#[test]
fn triangles_that_share_a_corner_stay_one_piece() {
    let a = [[0.5, 0.7, 2.0], [0.6, 0.7, 2.0], [0.5, 0.7, 2.1]];
    let b = [[0.6, 0.7, 2.0], [0.7, 0.7, 2.0], [0.6, 0.7, 2.1]];
    assert_eq!(split(lens(&[(a, 0), (b, 0)]), None).len(), 1);
}

#[test]
fn other_meshes_are_left_alone() {
    let mut mesh = lens(&[(strip(0.5, 2.0), 0)]);
    mesh.appearance.technique = "VehicleNFS13_Glass".into();
    assert_eq!(split(mesh.clone(), Some(&pattern())), vec![mesh]);
}

#[test]
fn a_dim_alpha_channel_is_read_against_its_own_peak() {
    let map = pattern();
    assert_eq!(map.level, [Some(89), Some(89), Some(89), Some(ALPHA_FLOOR)]);
    assert_eq!(map.lit_at([0.7, 0.5]), [false, false, false, true]);
    assert_eq!(map.lit_at([0.1, 0.5]), [true, false, false, false]);
}

#[test]
fn the_pattern_is_clamped_at_its_edge() {
    let map = pattern();
    // Past the right edge reads the last texel (dark), not a wrapped one (green).
    assert_eq!(map.lit_at([1.2, 0.5]), [false; 4]);
    assert_eq!(map.lit_at([-0.3, 0.5]), [true, false, false, false]);
}

#[test]
fn a_texture_without_alpha_has_no_tail_channel() {
    let texture = Texture {
        width: 4,
        height: 4,
        depth: 1,
        array_size: 1,
        mip_levels: 1,
        dxgi_format: 71,
        format: PixelFormat::Bc1,
        data: vec![0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0],
    };
    assert_eq!(EmissiveMap::new(&texture).unwrap().level[3], None);
}

#[test]
fn the_interiors_are_cut_by_circuit_and_side() {
    // Rear centre (reverse), rear left (brake), front left (head), a dark front triangle, and a rear triangle on the head
    // channel (not read at the rear).
    let mesh = interior(&[
        (strip(-0.05, -2.2), 2),
        (strip(0.6, -2.1), 0),
        (strip(0.6, 2.1), 1),
        (strip(0.3, 2.0), 4),
        (strip(0.3, -2.0), 1),
    ]);
    let groups = lamp_groups(&mesh, Some(&pattern())).unwrap();
    let found: Vec<_> = groups.iter().map(|g| (g.surface, g.triangles.clone())).collect();
    assert_eq!(found[0], (None, vec![3, 4]), "the triangles no circuit lights come first");
    assert!(found.contains(&(surface(LampEnd::Rear, LampSide::Left, LampCircuit::Brake), vec![1])));
    assert!(found.contains(&(surface(LampEnd::Front, LampSide::Left, LampCircuit::Head), vec![2])));
    assert!(found.contains(&(surface(LampEnd::Rear, LampSide::Centre, LampCircuit::Reverse), vec![0])));
    assert_eq!(groups.len(), 4);
}

#[test]
fn the_pieces_of_a_cut_are_meshes_of_their_own() {
    let mesh = interior(&[(strip(-0.05, -2.2), 2), (strip(0.3, 2.0), 4)]);
    let pieces = split(mesh, Some(&pattern()));
    assert_eq!(pieces.len(), 2);
    assert_eq!(pieces[0].appearance.kind, SurfaceKind::Light, "the rest keeps its look");
    assert!(matches!(pieces[1].appearance.kind, SurfaceKind::Lamp(s) if s.circuit == LampCircuit::Reverse));
    assert!(pieces.iter().all(|p| p.triangles.len() == 1 && p.vertices.len() == 3));
}

#[test]
fn interiors_with_nothing_lit_or_no_pattern_stay_one_mesh() {
    let mesh = interior(&[(strip(0.3, 2.0), 4)]);
    assert_eq!(split(mesh.clone(), Some(&pattern())), vec![mesh.clone()]);
    assert_eq!(split(mesh.clone(), None), vec![mesh]);
}

#[test]
fn a_lens_piece_is_cut_by_the_circuits_of_its_triangles() {
    // One connected rear strip: a brake triangle, a reverse triangle and a dark one that follows the larger share.
    let a = [[0.5, 0.7, -2.0], [0.6, 0.7, -2.0], [0.5, 0.7, -2.1]];
    let b = [[0.6, 0.7, -2.0], [0.7, 0.7, -2.0], [0.6, 0.7, -2.1]];
    let c = [[0.7, 0.7, -2.0], [0.8, 0.7, -2.0], [0.7, 0.7, -2.1]];
    let d = [[0.8, 0.7, -2.0], [0.9, 0.7, -2.0], [0.8, 0.7, -2.1]];
    let pieces = split(lens(&[(a, 0), (b, 2), (c, 2), (d, 4)]), Some(&pattern()));
    let found = surfaces(&pieces);
    assert_eq!(pieces.len(), 2);
    assert!(found.contains(&surface(LampEnd::Rear, LampSide::Left, LampCircuit::Brake)));
    assert!(found.contains(&surface(LampEnd::Rear, LampSide::Left, LampCircuit::Reverse)));
    let reverse = found.iter().position(|s| s.is_some_and(|s| s.circuit == LampCircuit::Reverse)).unwrap();
    assert_eq!(pieces[reverse].triangles.len(), 3, "the dark triangle follows the main circuit of its piece");
}

#[test]
fn a_front_lens_piece_ignores_a_rear_circuit() {
    let pieces = split(lens(&[(strip(0.5, 2.0), 2)]), Some(&pattern()));
    assert_eq!(surfaces(&pieces), vec![surface(LampEnd::Front, LampSide::Left, LampCircuit::Head)]);
}

/// The classes `body_parts` gives line up with the meshes `load` makes, lamp pieces included, on every car of the install
/// (`CHAMELEON_GAME_DIR`); and the 918 Spyder gets its reverse bar (blue channel, low at the rear centre) as lit pieces.
#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn the_body_classes_line_up_with_the_lamp_pieces_of_every_car() {
    use chameleon_bundle::{Bundle, Pool};
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let dir = std::path::Path::new(&dir);
    let textures = std::fs::read(dir.join("VEHICLES/VEHICLETEX.BNDL")).unwrap();
    let shaders = std::fs::read(dir.join("SHADERS.BNDL")).unwrap();
    let (mut cars, mut reverse_bar) = (0, false);
    for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
        let file = entry.file_name().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(entry.path()).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let mut pool = Pool::new();
        pool.add(Bundle::parse(&data).unwrap());
        pool.add(Bundle::parse(&textures).unwrap());
        pool.add(Bundle::parse(&shaders).unwrap());
        let spec = crate::find_spec(&bundle).unwrap();
        let vehicle = crate::load(&pool, spec).unwrap();
        let Some(skeleton) = crate::Skeleton::read(&bundle) else { continue };
        let parts = crate::hinged_parts(&bundle).unwrap();
        let classes = crate::body_parts(&pool, spec, &skeleton, &parts).unwrap();
        assert_eq!(classes.len(), vehicle.body.len(), "{file}");
        for (lod, (meshes, classes)) in vehicle.body.iter().zip(&classes).enumerate() {
            assert_eq!(meshes.len(), classes.len(), "{file} level {lod}");
            for (mesh, classes) in meshes.iter().zip(classes) {
                assert_eq!(mesh.triangles.len(), classes.len(), "{file} level {lod}");
            }
        }
        cars += 1;
        if file == "VEH_2196000_HI.BNDL" {
            reverse_bar = vehicle.body[0].iter().any(|mesh| {
                matches!(mesh.appearance.kind, SurfaceKind::Lamp(s) if s.circuit == LampCircuit::Reverse
                    && s.side == LampSide::Centre && s.end == LampEnd::Rear)
                    && mesh.vertices.iter().all(|v| v.position[1] < 0.4 && v.position[2] < -2.2)
            });
        }
    }
    assert!(cars > 50, "{cars} cars");
    assert!(reverse_bar, "the 918 Spyder has a lit reverse bar low at the rear centre");
}

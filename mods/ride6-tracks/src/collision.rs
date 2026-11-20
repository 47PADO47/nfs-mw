//! A cell's ground as the game's collision: the triangles in physics space with the surface of their material.

use blackbox_attrib::vlt_hash;
use blackbox_collision::SoupTriangle;

use crate::cells::GroundTriangle;
use crate::pack::Material;

/// The `simsurface` key of a surface name of the pack; a name the game has no collection for is looked up as `unknown`
/// there, so an unexpected name still drives like ground.
pub fn surface_hash(name: &str) -> u32 {
    vlt_hash(&name.to_ascii_lowercase())
}

/// A render-space point (x forward, y left, z up) in physics space (x right, y up, z forward): `(-y, z, x)`.
pub fn to_physics([x, y, z]: [f32; 3]) -> [f32; 3] {
    [-y, z, x]
}

/// The collision triangles of a cell's ground.
pub fn soup(ground: &[GroundTriangle], materials: &[Material]) -> Vec<SoupTriangle> {
    ground
        .iter()
        .map(|triangle| SoupTriangle {
            points: triangle.points.map(to_physics),
            surface: surface_hash(&materials[triangle.material].surface),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_surface_names_hash_to_the_keys_the_install_documents() {
        assert_eq!(surface_hash("asphalt"), 0x19DB_2F1E);
        assert_eq!(surface_hash("Grass"), 0x772F_B736);
        assert_eq!(surface_hash("gravel"), 0xC1C5_77D6);
        assert_eq!(surface_hash("sand"), 0x999A_CD78);
        assert_eq!(surface_hash("dirt"), 0xD929_E923);
        assert_eq!(surface_hash("concrete"), 0x6CA2_6F9B);
    }

    #[test]
    fn render_space_becomes_physics_space() {
        assert_eq!(to_physics([1.0, 2.0, 3.0]), [-2.0, 3.0, 1.0]);
    }

    #[test]
    fn a_cell_becomes_triangles_with_their_surface() {
        let materials = vec![Material { name: "m".into(), texture: None, color: [0; 3], surface: "grass".into() }];
        let ground = [GroundTriangle { points: [[0.0, 0.0, 1.0], [1.0, 0.0, 1.0], [0.0, 1.0, 1.0]], material: 0 }];
        let triangles = soup(&ground, &materials);
        assert_eq!(triangles[0].surface, 0x772F_B736);
        assert_eq!(triangles[0].points[1], [0.0, 1.0, 1.0]);
    }
}

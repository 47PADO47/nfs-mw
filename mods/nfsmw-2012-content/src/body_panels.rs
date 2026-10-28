//! The painted bodywork of a car whose two-colour packs leave its surfaces black (`two_colour.rs`).
//!
//! The Ariel Atom's panels (nose, floor, the wheel arches) are not paint surfaces: they are untextured `VehicleNFS13_Body`
//! surfaces whose material is a black diffuse colour with a dim specular colour and a smooth surface, and only its suspension
//! and a few trim pieces are paint (`docs/formats/chameleon-customization.md`). Drawn as the data has them the car is black
//! whatever pack or paint the player picks. A surface that looks like that (black, not a metal, not rough: the seats and the
//! chrome of the same car are rough or bright) takes the car's colour like a paint surface: the importer turns it into one.

use chameleon_vehicle::{Lod, PLAIN_BODY_TECHNIQUE, PaintBlend, PlainSurface, Sheet, SurfaceKind};

/// A diffuse colour at or under this (linear) is black.
const BLACK: f32 = 0.01;
/// A specular colour over this is a metal.
const METAL: f32 = 0.1;
/// A roughness over this is a matte plastic or a fabric, which no paint is.
const ROUGH: f32 = 0.6;
/// The technique name a surface that becomes paint carries.
const PAINT_TECHNIQUE: &str = "VehicleNFS13_BodyPaint";

/// Whether `surface` is a black, smooth, non-metal panel.
pub fn is_panel(surface: &PlainSurface) -> bool {
    let most = |colour: [f32; 3]| colour.into_iter().fold(0.0, f32::max);
    most(surface.diffuse) <= BLACK && most(surface.specular) <= METAL && surface.roughness <= ROUGH
}

/// Make the panels of every level of `body` paint surfaces that take the first colour of `sheet`. `surfaces` are the plain
/// surfaces of each level (`Source::plain_surfaces_of`); a mesh is told by its technique, its triangle count and its colour.
/// Returns how many meshes changed.
pub fn paint(body: &mut [Lod], surfaces: &[Vec<PlainSurface>], sheet: &Sheet) -> usize {
    let mut changed = 0;
    for (level, found) in body.iter_mut().zip(surfaces) {
        for mesh in level.iter_mut() {
            let appearance = &mut mesh.appearance;
            if appearance.kind != SurfaceKind::Body || appearance.technique != PLAIN_BODY_TECHNIQUE {
                continue;
            }
            let Some(colour) = appearance.diffuse_colour else { continue };
            let is_this =
                |surface: &&PlainSurface| surface.triangles == mesh.triangles.len() && surface.diffuse == colour;
            if !found.iter().filter(is_this).any(is_panel) {
                continue;
            }
            appearance.kind = SurfaceKind::Paint;
            appearance.technique = PAINT_TECHNIQUE.to_owned();
            appearance.diffuse_colour = None;
            for vertex in &mut mesh.vertices {
                vertex.uv = sheet.uv_of(PaintBlend::First, vertex.uv);
            }
            changed += 1;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use chameleon_gfx::Vertex;
    use chameleon_vehicle::{Appearance, PartMesh};

    use super::*;

    fn surface(triangles: usize, diffuse: f32, specular: f32, roughness: f32) -> PlainSurface {
        PlainSurface { triangles, diffuse: [diffuse; 3], specular: [specular; 3], roughness }
    }

    fn mesh(triangles: usize, diffuse: f32) -> PartMesh {
        PartMesh {
            vertices: vec![Vertex { position: [0.0; 3], color: [255; 4], uv: [5.0, 6.0] }; 3],
            triangles: vec![[0, 1, 2]; triangles],
            appearance: Appearance {
                kind: SurfaceKind::Body,
                technique: PLAIN_BODY_TECHNIQUE.to_owned(),
                diffuse_texture: None,
                diffuse_colour: Some([diffuse; 3]),
                alpha_test: false,
            },
        }
    }

    #[test]
    fn a_panel_is_black_smooth_and_not_a_metal() {
        assert!(is_panel(&surface(1, 0.006, 0.06, 0.518)), "the Atom's panels");
        assert!(is_panel(&surface(1, 0.0027, 0.026, 0.108)), "its wheel arches, with a clearcoat");
        assert!(!is_panel(&surface(1, 0.0, 0.027, 0.754)), "its seats are rough");
        assert!(!is_panel(&surface(1, 0.0, 0.88, 0.79)), "its chrome is a metal");
        assert!(!is_panel(&surface(1, 0.133, 0.28, 0.6)), "its frame is bronze");
    }

    #[test]
    fn only_the_panels_become_paint_and_take_the_first_colour() {
        let sheet = Sheet::flat();
        let mut body = vec![vec![mesh(4, 0.006), mesh(3, 0.0), mesh(2, 0.133)]];
        let found = vec![vec![surface(4, 0.006, 0.06, 0.5), surface(3, 0.0, 0.03, 0.75), surface(2, 0.133, 0.28, 0.6)]];
        assert_eq!(paint(&mut body, &found, &sheet), 1);
        assert_eq!(body[0][0].appearance.kind, SurfaceKind::Paint);
        assert_eq!(body[0][0].appearance.technique, PAINT_TECHNIQUE);
        assert_eq!(body[0][0].appearance.diffuse_colour, None);
        assert_eq!(body[0][0].vertices[2].uv, [0.25, 0.5]);
        assert_eq!(body[0][1].appearance.kind, SurfaceKind::Body, "the seats stay as they are");
        assert_eq!(body[0][1].vertices[2].uv, [5.0, 6.0]);
        assert_eq!(body[0][2].appearance.kind, SurfaceKind::Body, "the frame stays as it is");
    }

    #[test]
    fn a_mesh_the_data_does_not_know_stays_as_it_is() {
        let mut body = vec![vec![mesh(7, 0.006)]];
        assert_eq!(paint(&mut body, &[vec![surface(4, 0.006, 0.06, 0.5)]], &Sheet::flat()), 0);
        assert_eq!(paint(&mut body, &[], &Sheet::flat()), 0, "no surfaces at all");
        assert_eq!(body[0][0].appearance.kind, SurfaceKind::Body);
    }
}

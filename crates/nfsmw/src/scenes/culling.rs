//! NFS: Most Wanted's back-face culling rules: which effects cull blended surfaces, and which scenery instances
//! store their mirror in their vertices. See `docs/specs/backface-culling.md`.

use glam::Mat4;

/// The car effect (`CarShader`): it culls the back faces of blended surfaces (car glass) too. Every other effect
/// draws blended surfaces on both faces.
const CAR_EFFECT: u32 = 4;

/// `SceneryInstance` flag of a mirrored copy. Every instance whose rotation mirrors has it; the world-space copies
/// (`*_DEINST`) that have it carry the mirror in their vertices, so their triangles are wound the other way.
const MIRRORED_INSTANCE: u32 = 0x0040_0000;

/// Whether shading groups of effect `effect_id` cull blended surfaces (see `blackbox_scene::Sidedness`).
pub fn culls_blended(effect_id: u32) -> bool {
    effect_id == CAR_EFFECT
}

/// Whether a scenery instance with `flags` placed by `transform` shows its mesh wound the other way round than the
/// transform says (`blackbox_gfx::Instance::flip_winding`): mirrored by the flag but not by the transform.
pub fn flip_winding(flags: u32, transform: &Mat4) -> bool {
    (flags & MIRRORED_INSTANCE != 0) != (transform.determinant() < 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn only_the_car_effect_culls_blended_surfaces() {
        assert!(culls_blended(4));
        assert!(!culls_blended(0) && !culls_blended(5) && !culls_blended(6));
    }

    #[test]
    fn mirrored_copies_flip_only_when_the_transform_does_not_mirror() {
        let mirror = Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
        assert!(!flip_winding(0, &Mat4::IDENTITY));
        assert!(!flip_winding(MIRRORED_INSTANCE, &mirror), "the transform already mirrors");
        assert!(flip_winding(MIRRORED_INSTANCE, &Mat4::IDENTITY), "a world-space copy with the mirror baked in");
        assert!(flip_winding(0, &mirror));
    }
}

//! Calibrated Custom Glossy finish response, independent of the original GLOSS byte.
use blackbox_gfx::{GlossySurface, SurfaceKind};
use nfsmw_data::car::Paint;

pub(super) fn response(paint: &Paint) -> Option<GlossySurface> {
    let (roughness, coat, kind) = match paint.name.as_str() {
        n if n.starts_with("MATTE") => (0.52, 0.25, SurfaceKind::Legacy),
        n if n.starts_with("CHROME") => (0.12, 0.0, SurfaceKind::Conductor),
        n if n.starts_with("METAL") => (0.32, 0.9, SurfaceKind::Metallic),
        n if n.starts_with("PEARL") => (0.30, 0.9, SurfaceKind::Pearl),
        n if n.starts_with("GLOSS") => (0.24, 1.0, SurfaceKind::Coated),
        _ => return None,
    };
    Some(GlossySurface { roughness, coat, kind })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matte_is_rough_with_minimal_coat_and_authored_gloss_is_not_reinterpreted() {
        let mut paint = Paint { name: "MATTE01_PAINT".into(), rgb: [120; 3], gloss: 0, light_material: None };
        let matte = response(&paint).unwrap();
        paint.gloss = 255;
        assert_eq!(response(&paint), Some(matte));
        paint.name = "GLOSS_L1_COLOR01".into();
        let gloss = response(&paint).unwrap();
        assert!(matte.roughness > gloss.roughness && matte.coat < gloss.coat);
        paint.name = "UNKNOWN_MOD_PAINT".into();
        assert_eq!(response(&paint), None);
    }
}

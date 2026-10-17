//! Recolor the generated stock skin, leaving fixed liveries intact.

use anyhow::{Result, ensure};

use super::CarModel;

impl CarModel {
    /// Replace generated body textures while retaining the installed paint's gloss and material.
    pub fn set_body_color(&mut self, rgb: [u8; 3]) -> Result<()> {
        ensure!(self.paintable, "{} has a fixed livery", self.name);
        let paint = self.paint.as_mut().ok_or_else(|| anyhow::anyhow!("{} has no base paint", self.name))?;
        let skins: Vec<_> = self
            .textures
            .iter()
            .filter(|(_, texture)| {
                (texture.name.ends_with("_SKIN1") || texture.name == "ROOF_SKIN")
                    && texture.width == 4
                    && texture.height == 4
            })
            .map(|(hash, texture)| (*hash, texture.name.clone()))
            .collect();
        ensure!(!skins.is_empty(), "{} has no generated body skin", self.name);
        paint.rgb = rgb;
        for (hash, name) in skins {
            self.textures.insert(hash, paint.texture(&name));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Paint, TextureSwaps};
    use super::*;
    use std::collections::HashMap;

    fn model(paintable: bool) -> CarModel {
        let paint = Paint { name: "test".into(), rgb: [10, 20, 30], gloss: 42, light_material: Some(123) };
        let skin = paint.texture("TEST_SKIN1");
        CarModel {
            driving_lods: Default::default(),
            flare_profiles: Default::default(),
            window_tint_material: None,
            name: "TEST".into(),
            car_type: None,
            solids: HashMap::new(),
            placements: Vec::new(),
            textures: HashMap::from([(skin.name_hash, skin)]),
            swaps: TextureSwaps::default(),
            paint: Some(paint),
            paintable,
            light_materials: HashMap::new(),
            floor_height: 0.0,
            corners: None,
            wheel_setup: None,
        }
    }

    #[test]
    fn body_color_rewrites_texels_and_preserves_material_and_gloss() {
        let mut model = model(true);
        for name in ["SPOILER_SKIN1", "ROOF_SKIN"] {
            let texture = model.paint.as_ref().unwrap().texture(name);
            model.textures.insert(texture.name_hash, texture);
        }
        model.set_body_color([70, 80, 90]).unwrap();
        let paint = model.paint.unwrap();
        assert_eq!(paint.rgb, [70, 80, 90]);
        assert_eq!(paint.gloss, 42);
        assert_eq!(paint.light_material, Some(123));
        assert!(model.textures.values().all(|t| t.data.as_chunks::<4>().0.iter().all(|p| *p == [90, 80, 70, 255])));
    }

    #[test]
    fn fixed_livery_and_missing_skin_fail_without_mutating_paint() {
        let mut fixed = model(false);
        assert!(fixed.set_body_color([1, 2, 3]).is_err());
        assert_eq!(fixed.paint.unwrap().rgb, [10, 20, 30]);
        let mut missing = model(true);
        missing.textures.clear();
        assert!(missing.set_body_color([1, 2, 3]).is_err());
        assert_eq!(missing.paint.unwrap().rgb, [10, 20, 30]);
    }
}

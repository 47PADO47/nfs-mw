//! Authored visual target and mask; spec: `docs/specs/speedbreaker-visuals.md`.
use super::ordinates;
use blackbox_attrib::{Database, vlt_hash};
use blackbox_gfx::RgbaImage;
use blackbox_tpk::Texture;

#[derive(Debug, Clone, Copy)]
pub struct Profile {
    pub seconds: f32,
    pub curve: [f32; 4],
    pub radial_scale: f32,
    pub radial_offset: f32,
}

pub fn load(db: &Database) -> Option<Profile> {
    let target = db.collection_by_key(vlt_hash("visuallookeffect"), 0x90d0_6c71)?;
    let radial = db.collection("visuallookeffect", "uves_radialblur")?;
    let profile = Profile {
        seconds: target.get("length")?.as_f32()?,
        curve: ordinates(target.get("graph")?)?,
        radial_scale: target.get("radialblur_scale")?.as_f32()?,
        radial_offset: radial.get("radialblur_uvoffset")?.as_f32()?,
    };
    if !profile.seconds.is_finite()
        || profile.seconds <= 0.0
        || !profile.radial_scale.is_finite()
        || profile.radial_scale < 0.0
        || !profile.radial_offset.is_finite()
        || !(0.0..=0.1).contains(&profile.radial_offset)
    {
        return None;
    }
    Some(profile)
}

pub fn mask(textures: &[Texture]) -> Option<RgbaImage> {
    let texture = textures.iter().find(|t| t.name.eq_ignore_ascii_case("UVESVIGNETTE"))?;
    RgbaImage::new(texture.width, texture.height, blackbox_tpk::decode_rgba8(texture)?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires NFSMW_GAME_DIR; authored visual target and global vignette"]
    fn installed_speedbreaker_visual_profile_and_mask_load() {
        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        let profile = load(&db).expect("authored target and radial offset");
        assert!(profile.curve[0].abs() < 1e-6 && (profile.curve[3] - 1.0).abs() < 1e-6);
        assert!(profile.seconds > 0.0 && profile.radial_scale > 0.0 && profile.radial_offset > 0.0);
        let globals = crate::world::load_global_textures(&dir).unwrap();
        let image = mask(&globals.textures).expect("installed UVESVIGNETTE");
        assert!(image.width > 1 && image.height > 1);
        let center = image.pixel(image.width / 2, image.height / 2).unwrap();
        let corner = image.pixel(0, 0).unwrap();
        eprintln!("vignette {}x{} center{center:?} corner{corner:?}; {profile:?}", image.width, image.height);
        assert_eq!(center[1], 0, "the center has no additive vignette contribution");
        assert!(corner[1] > center[1], "the perimeter carries additive brightness");
        assert!(corner[2] < center[2], "blue supplies ordinary screen gain");
    }
}

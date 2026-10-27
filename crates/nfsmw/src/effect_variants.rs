//! The effect variants the player chose, as the scene takes them (`docs/modding/effect-variants.md`).
//!
//! Each kind of effect with variants (`modding::backends::variants`) has a field here: the profile of the chosen value when it
//! is a mod's, `None` for the game's own "Original". The mods' systems give the whole struct to the scene every frame
//! (`Scene::set_effect_variants`, made by `Bridge::effect_variants`); a profile is read once for each chosen value, so what the
//! scene holds is cheap to hand over again and a car spawned later gets it. A new kind adds a field here, its line in
//! `Bridge::effect_variants` and a use in `WorldScene::scene_set_effect_variants`.

use std::sync::Arc;

use blackbox_modding::content::{NitrousProfile, SparksProfile, TireSmokeProfile, WindProfile, XenonProfile};

/// The profiles of the variants in effect, one field per kind.
#[derive(Debug, Clone, Default)]
pub struct EffectVariants {
    /// The look and sound of the nitrous (`NITROUS/<value>/profile.toml`).
    pub nitrous: Option<Arc<NitrousProfile>>,
    /// The look of the tire smoke (`TIRESMOKE/<value>/profile.toml`).
    pub tire_smoke: Option<Arc<TireSmokeProfile>>,
    /// The collision sparks of the Original PC spark style (`SPARKS/<value>/profile.toml`).
    pub collision_sparks: Option<Arc<SparksProfile>>,
    /// The line sparks of the Restored spark style (`XENON/<value>/profile.toml`). Sparks only: no contrail, no streaks.
    pub xenon_sparks: Option<Arc<XenonProfile>>,
    /// The loops of the wind (`WIND/<value>/profile.toml`), played by the car's sound in place of the game's own wind.
    pub wind: Option<Arc<WindProfile>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_mod_every_kind_is_the_games_own() {
        let variants = EffectVariants::default();
        assert!(variants.nitrous.is_none() && variants.tire_smoke.is_none());
        assert!(variants.collision_sparks.is_none() && variants.xenon_sparks.is_none() && variants.wind.is_none());
    }

    #[test]
    fn a_copy_shares_the_profile_it_was_given() {
        let variants = EffectVariants { tire_smoke: Some(Arc::new(TireSmokeProfile::default())), ..Default::default() };
        let copy = variants.clone();
        let (a, b) = (variants.tire_smoke.unwrap(), copy.tire_smoke.unwrap());
        assert!(Arc::ptr_eq(&a, &b), "handing it over every frame costs a reference count");
    }
}

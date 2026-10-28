//! Scene setters for vehicle effects.
use super::WorldScene;

impl WorldScene {
    pub(super) fn scene_set_tire_effects(&mut self, smoke: bool, skid_marks: bool) {
        self.tire_effects = [smoke, skid_marks];
        if let Some(drive) = self.players.lead_mut() {
            drive.effects.set_enabled(smoke, skid_marks);
        }
    }

    pub(super) fn scene_set_spark_style(&mut self, style: crate::settings::SparkStyle) {
        if self.spark_style != style {
            self.impact_fx.clear();
        }
        self.spark_style = style;
        if let Some(drive) = self.players.lead_mut() {
            drive.vehicle_effects.set_style(style);
        }
    }

    /// The effects of the nitrous variant the player chose: the flames take its tint, the screen its fades, the sound its
    /// samples (`listened_car_state`). `None` is the game's own, and nothing here changes then.
    pub(super) fn scene_set_nitrous_fx(
        &mut self,
        fx: Option<std::sync::Arc<blackbox_modding::content::NitrousProfile>>,
    ) {
        let flame = fx.as_deref().and_then(|profile| profile.flame);
        if let Some(drive) = self.players.lead_mut() {
            drive.flames.set_profile(flame);
        }
        if let Some(drive) = self.players.lead_mut() {
            drive.flames.set_effects(fx.clone());
        }
        self.nitrous_fx = fx;
    }

    /// The effect variants the player chose (`docs/modding/effect-variants.md`), given every frame: the nitrous as it always was,
    /// the tire smoke takes its profile and loads its pictures at the next frame (`maintain_tire_smoke`). `None` is the game's
    /// own, and nothing here changes then. Another kind of variant adds its own lines here, and a field of the scene when a car
    /// spawned later has to get it too.
    pub(super) fn scene_set_effect_variants(&mut self, variants: &crate::effect_variants::EffectVariants) {
        self.scene_set_nitrous_fx(variants.nitrous.clone());
        self.tire_smoke_variant = variants.tire_smoke.clone();
        self.wind_fx = variants.wind.clone();
        if let Some(drive) = self.players.lead_mut() {
            drive.effects.set_variant(variants.tire_smoke.clone());
            drive.vehicle_effects.set_spark_variant(variants.xenon_sparks.clone());
        }
        // Each spark variant plays in its own spark style: the collision sparks replace the install's particle groups of the
        // Original PC style, the restored sparks (given to the car above) the line sparks of the Restored style. The other style
        // keeps the install's.
        let original = self.spark_style == crate::settings::SparkStyle::OriginalPc;
        self.impact_fx.set_variant(variants.collision_sparks.clone().filter(|_| original));
    }

    pub(super) fn scene_set_exhaust_flames(&mut self, on: bool) {
        self.exhaust_flames = on;
        if let Some(drive) = self.players.lead_mut() {
            drive.flames.set_enabled(on);
        }
    }

    pub(super) fn scene_set_smoke_quality(&mut self, quality: crate::settings::SmokeQuality) {
        self.smoke_quality = quality;
        if let Some(drive) = self.players.lead_mut() {
            drive.effects.set_quality(quality);
        }
    }
}

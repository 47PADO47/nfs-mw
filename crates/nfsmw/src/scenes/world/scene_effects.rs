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

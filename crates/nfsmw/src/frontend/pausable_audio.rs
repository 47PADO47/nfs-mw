//! Audio forwarding while the world is paused or concealed.
use super::Pausable;

impl Pausable {
    pub(super) fn paused_car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        if self.paused.get() || self.concealed() {
            return None;
        }
        self.inner.car_sound()
    }

    pub(super) fn paused_second_car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        if self.paused.get() || self.concealed() {
            return None;
        }
        self.inner.second_car_sound()
    }

    pub(super) fn paused_take_common_sounds(&mut self) -> Vec<crate::audio::CommonEvent> {
        let sounds = self.inner.take_common_sounds();
        if self.paused.get() || self.concealed() {
            return Vec::new();
        }
        sounds
    }

    pub(super) fn paused_pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        if self.concealed() {
            return None;
        }
        self.inner.pursuit_feed()
    }

    pub(super) fn paused_roam_feed(&mut self) -> Option<crate::audio::RoamFeed> {
        if self.concealed() {
            return None;
        }
        self.inner.roam_feed()
    }

    pub(super) fn paused_take_traffic_sounds(&mut self) -> Vec<crate::audio::traffic::TrafficSound> {
        let sounds = self.inner.take_traffic_sounds();
        if self.concealed() {
            return Vec::new();
        }
        sounds
    }

    pub(super) fn paused_rotor_frame(&mut self, dt: f32) -> Option<crate::audio::HeliFrame> {
        let frame = self.inner.rotor_frame(dt);
        if self.concealed() {
            return None;
        }
        frame
    }

    pub(super) fn paused_siren_frame(&mut self, dt: f32) -> Option<crate::audio::SirenFrame> {
        let frame = self.inner.siren_frame(dt);
        if self.concealed() {
            return None;
        }
        frame
    }
}

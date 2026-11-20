//! Read-only native camera inputs from the current or presented player car.

use nfsmw_data::car::physics::SurfaceTable;

use super::Drive;
use crate::audio::{CarContact, ContactSource};
use crate::viewer::camera::original::{Avoidable, BodyInput, ImpactKind, NoiseControls};

impl Drive {
    pub fn camera_looking_back(&self, mode: crate::settings::CameraMode) -> bool {
        if crate::viewer::camera::original::index_of(mode).is_some() {
            return self.chase.look_back_held();
        }
        self.looking_back()
    }

    /// Current custom shot with shake, before world clearance.
    pub fn custom_camera(&self) -> crate::viewer::camera::ChaseView {
        self.pursuit.shot(self.chase.sample(1.0))
    }

    pub fn original_body(&self, interpolate: bool) -> Option<BodyInput> {
        if self.request.is_some() {
            return None;
        }
        let mut body = self.sim.as_ref()?.original_camera_body(self.speedbreaker_zoom(), self.chase.look_back_held());
        if !interpolate {
            // The rider's body while he is thrown (`rider_cam.rs`).
            self.blend_rider_body(&mut body, None);
            return Some(body);
        }
        let shown = self.pose();
        let rotation = shown.rotation * self.current.rotation.conjugate();
        body.position = shown.position + rotation * (body.position - self.current.position);
        body.forward = rotation * body.forward;
        body.up = rotation * body.up;
        self.blend_rider_body(&mut body, Some(self.alpha));
        Some(body)
    }

    pub fn original_noise(&self, surfaces: &SurfaceTable) -> NoiseControls {
        let sim = self.sim.as_ref().filter(|_| self.request.is_none());
        NoiseControls {
            terrain: sim.and_then(|car| car.surface_tag()).map_or([0.0; 2], |tag| surfaces.camera_noise(tag)),
            brake: sim.is_some_and(|car| car.original_camera_braking()),
            drag: false,
            overrev: sim.is_some_and(|car| car.original_camera_overrev()),
        }
    }

    pub fn original_step(&self) -> u32 {
        self.steps
    }

    /// Contacts from `original_step`; the timeline must consume each step once.
    pub fn original_impacts(&self) -> Vec<(ImpactKind, f32)> {
        if self.request.is_some() || self.sim.is_none() {
            return Vec::new();
        }
        // The bike's contacts (it slides and tumbles behind him) are not what the camera on the thrown rider looks at: they fade with its
        // attention (`docs/specs/ride6-feel-round14.md`, section 1).
        let contacts = self.sound.camera_contacts().iter().copied().filter_map(camera_impact);
        faded(contacts, 1.0 - self.rider_cam.weight())
    }

    pub fn camera_avoidable(&self) -> Option<Avoidable> {
        if self.request.is_some() {
            return None;
        }
        Some(self.sim.as_ref()?.camera_avoidable())
    }
}

/// The impacts with their closing speeds times `keep` (0 to 1); the ones that come to nothing are dropped.
fn faded(impacts: impl Iterator<Item = (ImpactKind, f32)>, keep: f32) -> Vec<(ImpactKind, f32)> {
    impacts.map(|(kind, speed)| (kind, speed * keep)).filter(|&(_, speed)| speed > 0.0).collect()
}

fn camera_impact(contact: CarContact) -> Option<(ImpactKind, f32)> {
    let closing = contact.closing_speed();
    if !closing.is_finite() || closing <= 0.0 {
        return None;
    }
    let kind = match contact.source {
        ContactSource::World => ImpactKind::World,
        ContactSource::Ground => ImpactKind::Ground,
        ContactSource::Object => ImpactKind::Object,
    };
    Some((kind, closing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn contact(source: ContactSource, velocity: Vec3) -> CarContact {
        CarContact {
            sound_profile: 0,
            vehicle: None,
            audio_zone: None,
            detached: false,
            source,
            identity: 1,
            surface: 0,
            point: Vec3::ZERO,
            normal: -Vec3::Z,
            velocity,
            delta_v: 100.0,
            front: false,
            bottom: false,
        }
    }

    #[test]
    fn native_glancing_contact_retains_only_normal_closing_speed() {
        // Native solver vectors separate this 30 m/s slide from its 5 m/s closing component.
        let (kind, speed) = camera_impact(contact(ContactSource::World, Vec3::new(30.0, 0.0, 5.0))).unwrap();
        assert!(matches!(kind, ImpactKind::World));
        assert_eq!(speed, 5.0);
        let (kind, speed) = camera_impact(contact(ContactSource::Ground, Vec3::new(15.0, 0.0, 30.0))).unwrap();
        assert!(matches!(kind, ImpactKind::Ground));
        assert_eq!(speed, 30.0);
        let (kind, speed) = camera_impact(contact(ContactSource::Object, Vec3::new(0.0, 0.0, 40.0))).unwrap();
        assert!(matches!(kind, ImpactKind::Object));
        assert_eq!(speed, 40.0);
    }

    #[test]
    fn separating_or_pure_sliding_contacts_do_not_start_camera_timers() {
        for velocity in [Vec3::new(30.0, 0.0, 0.0), Vec3::new(30.0, 0.0, -5.0)] {
            assert!(camera_impact(contact(ContactSource::World, velocity)).is_none());
        }
    }

    #[test]
    fn the_bikes_contacts_fade_with_the_cameras_attention_to_its_rider() {
        let hits = || vec![(ImpactKind::Ground, 20.0), (ImpactKind::World, 6.0)].into_iter();
        assert_eq!(faded(hits(), 1.0).len(), 2, "on the bike: all of them, as they were");
        let half = faded(hits(), 0.5);
        assert_eq!(half.iter().map(|&(_, s)| s).collect::<Vec<_>>(), [10.0, 3.0]);
        assert!(faded(hits(), 0.0).is_empty(), "on the rider the bike sliding behind him shakes nothing");
    }
}

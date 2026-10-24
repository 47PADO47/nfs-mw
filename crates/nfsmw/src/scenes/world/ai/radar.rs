//! Original ground detector: nearest live unit within a 300-metre, 45-degree 3D cone.
//! Contract: `docs/specs/hud-pursuit-state.md`.

use glam::Vec3;

use super::TrafficWorld;
use crate::hud::{RadarHud, police::RADAR_RANGE};
use crate::scenes::world::space;

impl TrafficWorld {
    pub fn radar(&self, position: Vec3, forward: Vec3) -> Option<RadarHud> {
        if self.is_pursuing() && self.police.cooldown_status() != blackbox_pursuit::CooldownStatus::CoolDown {
            return None;
        }
        let at = Vec3::from_array(space::to_physics(position));
        let fwd = Vec3::from_array(space::to_physics(forward));
        self.cars
            .iter()
            .filter(|car| car.is_live_police())
            .map(|car| car.physics_position() - at)
            .filter_map(|delta| target(delta, fwd))
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }

    pub fn vehicle_heat(&self) -> f32 {
        self.police.vehicle_heat()
    }

    pub fn cops_engaged(&self) -> u32 {
        self.cars.iter().filter(|car| car.is_live_police() && !car.is_patrol()).count() as u32
    }

    pub fn cops_damaged(&self) -> u32 {
        self.pursuit.as_ref().map_or_else(|| self.police.damaged_count(), |pursuit| pursuit.damaged.len() as u32)
    }
}

fn target(delta: Vec3, forward: Vec3) -> Option<RadarHud> {
    let distance = delta.length();
    if !(0.0..=RADAR_RANGE).contains(&distance) || distance == 0.0 {
        return None;
    }
    let forward = forward.try_normalize()?;
    let direction = delta / distance;
    let dot = forward.dot(direction).clamp(-1.0, 1.0);
    if dot < std::f32::consts::FRAC_1_SQRT_2 {
        return None;
    }
    let magnitude = dot.acos();
    let angle = match forward.cross(Vec3::Y).dot(direction) < 0.0 {
        true => -magnitude,
        false => magnitude,
    };
    Some(RadarHud { angle, distance })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detector_uses_the_forward_cone_instead_of_all_directions() {
        assert!(target(Vec3::Z * 100.0, Vec3::Z).is_some());
        assert!(target(-Vec3::Z * 10.0, Vec3::Z).is_none());
        assert!(target(Vec3::X * 10.0, Vec3::Z).is_none());
        assert!(target(Vec3::new(100.0, 0.0, 100.0), Vec3::Z).is_some(), "45-degree edge is included");
        assert!(target(Vec3::new(101.0, 0.0, 100.0), Vec3::Z).is_none());
    }

    #[test]
    fn detector_uses_full_3d_range_without_an_overpass_height_gate() {
        let elevated = target(Vec3::new(0.0, 180.0, 240.0), Vec3::Z).unwrap();
        assert_eq!(elevated.distance, RADAR_RANGE, "the 300-metre boundary is included");
        assert_eq!(elevated.normalized_range(), 1.0);
        assert!(target(Vec3::new(0.0, 180.0, 240.0) * 1.001, Vec3::Z).is_none());
        assert!(target(Vec3::new(0.0, 120.0, 100.0), Vec3::Z).is_none(), "elevation still contributes to cone angle");
    }

    #[test]
    fn detector_heading_and_signed_polar_direction_are_consistent() {
        let left = target(Vec3::new(-10.0, 0.0, 100.0), Vec3::Z).unwrap();
        let right = target(Vec3::new(10.0, 0.0, 100.0), Vec3::Z).unwrap();
        assert!(left.angle > 0.0 && right.angle < 0.0);
        assert!((left.angle + right.angle).abs() < 1e-6);
        let turned = target(Vec3::X * 100.0, Vec3::X * 5.0).unwrap();
        assert_eq!(turned.angle, 0.0, "heading is normalized");
        assert!(target(Vec3::Z * 100.0, Vec3::X).is_none());
        let tilted = target(Vec3::new(0.0, 100.0, 100.0), Vec3::new(0.0, 1.0, 1.0)).unwrap();
        assert!(tilted.angle.abs() < 0.001, "player pitch participates in the cone");
        assert!(target(Vec3::Z, Vec3::ZERO).is_none());
    }
}

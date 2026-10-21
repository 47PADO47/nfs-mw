//! Pursuit breaker zones (spec `docs/specs/pursuit-breakers.md`, "Cop zones"): a breaker event leaves an expiring disc
//! on the road plane. Every police vehicle that is inside a disc while it lives is destroyed, including one that
//! enters later. The test is horizontal, so the height does not count.

use glam::Vec2;

/// Whether `point` (physics x and z) is inside the zone of `radius` around `centre`. The edge is outside, as the spec
/// says "below the zone radius".
pub fn inside(centre: Vec2, radius: f32, point: Vec2) -> bool {
    point.distance(centre) < radius
}

/// One live zone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    pub centre: Vec2,
    pub radius: f32,
    /// Seconds left.
    pub remaining: f32,
}

/// The live zones. A zone is applied on every tick up to and including the one on which it expires, then dropped.
#[derive(Debug, Clone, Default)]
pub struct Zones {
    zones: Vec<Zone>,
}

impl Zones {
    /// Adds a zone. A zone with no radius or no duration never exists.
    pub fn push(&mut self, centre: Vec2, radius: f32, duration: f32) {
        if !(radius > 0.0 && duration > 0.0) {
            return;
        }
        self.zones.push(Zone { centre, radius, remaining: duration });
    }

    /// Whether a live zone covers `point`.
    pub fn covers(&self, point: Vec2) -> bool {
        self.zones.iter().any(|z| inside(z.centre, z.radius, point))
    }

    /// Ends the tick of `dt` seconds: zones that ran out are dropped.
    pub fn step(&mut self, dt: f32) {
        for zone in &mut self.zones {
            zone.remaining -= dt;
        }
        self.zones.retain(|z| z.remaining > 0.0);
    }

    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }

    pub fn len(&self) -> usize {
        self.zones.len()
    }

    /// The live zones, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &Zone> + '_ {
        self.zones.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cop_inside_the_radius_is_stopped() {
        assert!(inside(Vec2::ZERO, 12.0, Vec2::new(10.0, 0.0)));
    }

    #[test]
    fn the_edge_is_outside_the_zone() {
        assert!(!inside(Vec2::ZERO, 12.0, Vec2::new(12.0, 0.0)));
    }

    #[test]
    fn a_zone_is_a_disc_on_the_road_plane() {
        assert!(inside(Vec2::new(5.0, 5.0), 3.0, Vec2::new(7.0, 6.0)));
        assert!(!inside(Vec2::new(5.0, 5.0), 3.0, Vec2::new(9.0, 5.0)));
    }

    #[test]
    fn a_zone_covers_later_entrants_until_it_expires() {
        let mut zones = Zones::default();
        zones.push(Vec2::ZERO, 10.0, 5.0);
        assert!(!zones.covers(Vec2::new(30.0, 0.0)));
        zones.step(4.9);
        assert!(zones.covers(Vec2::new(9.0, 0.0)), "a cop that arrives at 4.9 s is still caught");
        zones.step(0.2);
        assert!(zones.is_empty());
        assert!(!zones.covers(Vec2::new(9.0, 0.0)));
    }

    #[test]
    fn a_zone_without_a_radius_or_a_duration_is_ignored() {
        let mut zones = Zones::default();
        zones.push(Vec2::ZERO, 0.0, 5.0);
        zones.push(Vec2::ZERO, 5.0, 0.0);
        zones.push(Vec2::ZERO, f32::NAN, 5.0);
        assert!(zones.is_empty());
    }
}

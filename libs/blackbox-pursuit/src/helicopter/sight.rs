//! Whether the helicopter can see the player (`CanSeeTarget`, spec §3). The visual sphere is near; farther out the
//! helicopter sees only the half space in front of it, and never through the world (a clear-line test the caller
//! provides), nor from inside a tunnel. A hidden zone latches off the sight until it is left.

use glam::Vec3;

/// Within this distance (m) the helicopter sees the player whatever its direction.
pub const VISUAL_SPHERE: f32 = 75.0;
/// Within this distance (m) of where the player was last seen, the helicopter keeps chasing for a while.
pub const GRACE_DISTANCE: f32 = 150.0;
/// The grace applies while the player is within this many metres (squared is 400) of its last seen place.
pub const GRACE_LAST_SEEN: f32 = 20.0;

/// What the helicopter knows about its target this think.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub heli: Vec3,
    pub forward: Vec3,
    pub target: Vec3,
    /// `heliLOSdistance` of the level: how far the helicopter sees to its front, metres.
    pub range: f32,
    /// The target is in a zone hidden from helicopters.
    pub hidden: bool,
    /// The game camera is in a tunnel.
    pub in_tunnel: bool,
}

/// The sight state carried between thinks: the hidden latch and the place the player was last seen.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Sight {
    latched: bool,
    last_seen: Option<Vec3>,
}

impl Sight {
    /// Whether the helicopter sees its target now. `clear` tests the line from the helicopter to the target, lifted
    /// a metre; it is only called when the target is in view by distance and direction.
    pub fn can_see(&mut self, look: &Look, clear: impl FnOnce() -> bool) -> bool {
        if look.hidden && self.latched {
            return false;
        }
        self.latched = false;
        let to = look.target - look.heli;
        let dist = to.length();
        let in_sight = self.in_view(look, dist, to, clear);
        if in_sight {
            return true;
        }
        if look.hidden {
            self.latched = true;
            return false;
        }
        self.grace(look, dist)
    }

    /// The direct test: near enough, or in front and within range, with a clear line and out of a tunnel.
    fn in_view(&mut self, look: &Look, dist: f32, to: Vec3, clear: impl FnOnce() -> bool) -> bool {
        let near = dist <= VISUAL_SPHERE;
        let ahead = dist < look.range && look.forward.dot(to.normalize_or_zero()) > 0.0;
        if !(near || ahead) {
            return false;
        }
        if look.in_tunnel || !clear() {
            return false;
        }
        self.last_seen = Some(look.target);
        true
    }

    /// Just lost: keeps chasing when the player is close to where it was last seen.
    fn grace(&self, look: &Look, dist: f32) -> bool {
        let Some(seen) = self.last_seen else { return false };
        dist < GRACE_DISTANCE && (look.target - seen).length_squared() < GRACE_LAST_SEEN * GRACE_LAST_SEEN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(target: Vec3) -> Look {
        Look { heli: Vec3::ZERO, forward: Vec3::X, target, range: 251.0, hidden: false, in_tunnel: false }
    }

    #[test]
    fn sees_anything_within_the_visual_sphere() {
        let mut sight = Sight::default();
        assert!(sight.can_see(&look(Vec3::new(-50.0, 0.0, 0.0)), || true));
    }

    #[test]
    fn beyond_the_sphere_only_the_front_half_space_counts() {
        let mut sight = Sight::default();
        assert!(sight.can_see(&look(Vec3::new(120.0, 0.0, 0.0)), || true));
        let mut sight = Sight::default();
        assert!(!sight.can_see(&look(Vec3::new(-120.0, 0.0, 0.0)), || true), "behind it, far");
    }

    #[test]
    fn beyond_the_range_nothing_is_seen_even_in_front() {
        let mut sight = Sight::default();
        assert!(!sight.can_see(&look(Vec3::new(300.0, 0.0, 0.0)), || true));
    }

    #[test]
    fn a_blocked_line_or_a_tunnel_hides_the_target() {
        let mut sight = Sight::default();
        assert!(!sight.can_see(&look(Vec3::new(40.0, 0.0, 0.0)), || false));
        let tunnel = Look { in_tunnel: true, ..look(Vec3::new(40.0, 0.0, 0.0)) };
        assert!(!sight.can_see(&tunnel, || true));
    }

    #[test]
    fn a_hidden_zone_latches_the_sight_off_until_it_is_left() {
        let mut sight = Sight::default();
        let target = Vec3::new(160.0, 0.0, 0.0);
        let hidden = Look { hidden: true, ..look(target) };
        assert!(!sight.can_see(&hidden, || false), "hidden and out of the line: the latch sets");
        assert!(!sight.can_see(&hidden, || true), "still latched inside the zone, even with a clear line");
        assert!(sight.can_see(&look(Vec3::new(60.0, 0.0, 0.0)), || true), "out of the zone: seen again");
    }

    #[test]
    fn a_visible_target_in_a_hidden_zone_is_still_seen_before_the_latch_sets() {
        let mut sight = Sight::default();
        let hidden = Look { hidden: true, ..look(Vec3::new(40.0, 0.0, 0.0)) };
        assert!(sight.can_see(&hidden, || true), "the latch only applies once it has set");
    }

    #[test]
    fn grace_keeps_a_lost_target_near_where_it_was_seen() {
        let mut sight = Sight::default();
        assert!(sight.can_see(&look(Vec3::new(40.0, 0.0, 0.0)), || true));
        let blocked = look(Vec3::new(50.0, 0.0, 0.0));
        assert!(sight.can_see(&blocked, || false), "blocked but near the last seen place");
        let gone = look(Vec3::new(40.0, 0.0, 200.0));
        assert!(!sight.can_see(&gone, || false), "far from the last seen place");
    }
}

//! The first cop of a chase that starts with no unit: the forced start (`SpawnPursuitCar`, spec `ai-pursuit-cops.md`
//! section 4.1). A road cursor is put under the player (a traffic road off the junction connectors, a connector only
//! when no road is near), heading the way the player faces, and walked a set distance through the junctions ahead; the
//! car stands there at rest: 340 m ahead facing on along the road and inside the spawn window, or, when the start
//! follows a jump to a bounty place or a milestone, 80 m ahead turned to face the player with no window. When no spot
//! comes of it, the car goes 15 m behind the player (`SpawnCopCarNow`).

use blackbox_roads::Body;

use super::*;

/// How far ahead of the player the first cop of a forced start goes, metres along the road.
const AHEAD: f32 = 340.0;
/// The same after a jump to a bounty place or a milestone's start.
const AFTER_JUMP: f32 = 80.0;
/// Where the fallback puts the car, metres behind the player along the road.
const BEHIND: f32 = 15.0;

/// Which forced-start placement the chase's first cop takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::scenes::world::ai) enum Opening {
    /// A chase started from where the player drives (the console, F5 or a scripted start).
    Ahead,
    /// A chase started where a bounty or milestone jump put the player.
    AfterJump,
}

impl TrafficWorld {
    /// Whether the next chase starts where a jump to a bounty place or a milestone put the player: its first cop then
    /// comes 80 m ahead, facing the player. The chase's start takes the mark.
    pub fn set_jump_start(&mut self, jumped: bool) {
        self.jump_start = jumped;
    }

    /// After the start's patrols joined: a chase with none of them places its first cop by the forced-start rule.
    pub(super) fn arm_opening(&mut self) {
        let after_jump = std::mem::take(&mut self.jump_start);
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        if pursuit.involved > 0 {
            return;
        }
        pursuit.opening = Some(match after_jump {
            true => Opening::AfterJump,
            false => Opening::Ahead,
        });
    }

    /// A chase cop of `name` by the placement the chase is at: the forced start's while no cop has joined, else the
    /// reinforcement walk.
    pub(super) fn spawn_chase_cop(
        &mut self,
        name: &str,
        focus: Focus,
        target: Vec3,
        player: Performance,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let Some(opening) = self.pursuit.as_ref().and_then(|pursuit| pursuit.opening) else {
            return self.spawn_cop(name, focus, target, player, collision, surfaces);
        };
        let Some(body) = focus.body else { return false };
        if !self.has_cop_model(name) {
            return false;
        }
        let band = match opening {
            Opening::Ahead => Some((MIN_DISTANCE, MAX_DISTANCE)),
            Opening::AfterJump => None,
        };
        let spot = |nav: &RoadNav, band| CopSpot { at: nav.position, target, speed: 0.0, player, band };
        if let Some(nav) = self.opening_cursor(&body, opening)
            && self.spawn_cop_on_road(name, spot(&nav, band), nav, collision, surfaces)
        {
            return true;
        }
        let Some(nav) = self.behind_cursor(&body) else { return false };
        self.spawn_cop_on_road(name, spot(&nav, None), nav, collision, surfaces)
    }

    /// The road cursor under the player, heading the way `heading` points: on a traffic road off the junction
    /// connectors, or on a connector when no road is near; it then walks through junctions as any cursor does.
    fn player_cursor(&self, body: &Body, heading: Vec3) -> Option<RoadNav> {
        for no_decision in [true, false] {
            let filter = SegmentFilter { traffic: true, no_decision, ..SegmentFilter::default() };
            let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, filter);
            if !nav.init_at_point(&self.network, &self.index, body.position, heading, false) {
                continue;
            }
            nav.filter.no_decision = false;
            return Some(nav);
        }
        None
    }

    /// The forced start's spot: the walk ahead of the player, turned to face back after a jump.
    pub(in crate::scenes::world::ai) fn opening_cursor(&mut self, body: &Body, opening: Opening) -> Option<RoadNav> {
        let forward = Vec3::new(body.forward.x, 0.0, body.forward.y);
        let mut nav = self.player_cursor(body, forward)?;
        let distance = match opening {
            Opening::Ahead => AHEAD,
            Opening::AfterJump => AFTER_JUMP,
        };
        nav.advance(&self.network, distance, forward, &mut self.rng.police);
        if opening == Opening::AfterJump {
            nav.reverse(&self.network);
        }
        Some(nav).filter(|nav| nav.position.is_finite())
    }

    /// The fallback: 15 m behind the player, facing the way the player faces.
    fn behind_cursor(&mut self, body: &Body) -> Option<RoadNav> {
        let back = -Vec3::new(body.forward.x, 0.0, body.forward.y);
        let mut nav = self.player_cursor(body, back)?;
        nav.advance(&self.network, BEHIND, back, &mut self.rng.police);
        nav.reverse(&self.network);
        Some(nav).filter(|nav| nav.position.is_finite())
    }
}

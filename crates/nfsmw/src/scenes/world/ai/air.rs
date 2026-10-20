//! Air support in the cop manager: the helicopter takes its turn among the cop types of the wave, and appears on the cop
//! road 250 m behind the target, 20 m up. Spec: `docs/specs/ai-helicopter.md` section 2.
//!
//! The original asks the pursuit for the next cop type (`CopRequest`); the helicopter (`copheli`, one wanted) is one of
//! the wave's types, picked by its weight among the types the chase still lacks. It is allowed once its timer is spent
//! (10 s at the start of the chase, then `TimeBetweenHeliActive` after one left) and the support row's minimum delay has
//! passed; the search-mode roll can force it. `Police::heli_ask` holds those rules; this file holds the wave's side.

use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter, Body};
use glam::Vec3;
use nfsmw_data::pursuit::HELICOPTER;

use super::TrafficWorld;
use super::police::heli::{Ask, HeliStatus, Spawn};

/// How far above the nav's point the helicopter appears (`y += 20`).
const SPAWN_HEIGHT: f32 = 20.0;

impl TrafficWorld {
    /// At a cop spawn opportunity: whether the helicopter is the type picked. If so its spawn is queued for the police
    /// (they place it on their next step) and the chase counts it as involved. `ground_wanted` says whether the wave still
    /// wants ground cars, which share the pick with it.
    pub(super) fn pick_helicopter(&mut self, player: &Body, ground_wanted: bool) -> bool {
        let Some(pursuit) = self.pursuit.as_ref() else { return false };
        let Some(level) = self.police.level() else { return false };
        let weight = pursuit.row().and_then(|row| row.cops.iter().find(|c| c.name == HELICOPTER)).map(|c| c.chance.max(1));
        let have = |name: &str| self.cars.iter().filter(|c| c.is_cop() && c.name.eq_ignore_ascii_case(name)).count() as u32;
        let ground: u32 = match ground_wanted {
            true => pursuit.wave().filter(|&(name, count, _)| have(name) < count).map(|(_, _, chance)| chance.max(1)).sum(),
            false => 0,
        };
        let search_roll = self.rng.next_f32();
        let pick_roll = self.rng.next_f32();
        let picked = match self.police.heli_ask(level, search_roll) {
            Ask::No => false,
            Ask::Forced => true,
            Ask::Allowed => weight.is_some_and(|w| pick_roll * ((w + ground) as f32) < w as f32),
        };
        if !picked {
            return false;
        }
        let spawn = self.heli_spawn_point(player, self.police.heli_spawn_distance(level));
        self.police.queue_helicopter(spawn);
        if let Some(pursuit) = self.pursuit.as_mut() {
            pursuit.sent_helicopter();
        }
        log::info!("pursuit: the helicopter is sent, {:.0} m behind the player", self.police.heli_spawn_distance(level));
        true
    }

    /// Where the helicopter appears (`SpawnPursuitHelicopter`): `distance` metres along the cop road behind the target, or
    /// at the target when there is no cop road there, 20 m up, facing along the road.
    fn heli_spawn_point(&mut self, player: &Body, distance: f32) -> Spawn {
        let forward = Vec3::new(player.forward.x, 0.0, player.forward.y).normalize_or_zero();
        let back = -forward;
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter { cop: true, ..SegmentFilter::default() });
        let (mut position, facing) = match nav.init_at_point(&self.network, &self.index, player.position, back, false) {
            true => {
                nav.advance(&self.network, distance, back, &mut self.rng);
                (nav.position, nav.forward.with_y(0.0).normalize_or_zero())
            }
            false => (player.position, forward),
        };
        position.y += SPAWN_HEIGHT;
        Spawn { position, heading: facing.z.atan2(facing.x) }
    }

    /// The helicopter's state for the speech and the HUD: whether it has the player in sight, its fuel, its mode and its
    /// health. `None` while no helicopter flies.
    pub fn helicopter_status(&self) -> Option<HeliStatus> {
        self.police.helicopter_status()
    }

    /// The helicopter's position, nose direction and velocity in render space, for the rotor sound.
    pub fn helicopter_motion(&self) -> Option<(Vec3, Vec3, Vec3)> {
        let (position, forward, velocity) = self.police.helicopter_motion()?;
        let render = |v: Vec3| super::super::space::to_render(v.to_array());
        Some((render(position), render(forward), render(velocity)))
    }

    /// A pursuit breaker's zone destroys the helicopter above it: only the horizontal distance counts, height is ignored
    /// (spec `ai-pursuit-roadblocks.md` section 7).
    pub(super) fn apply_breakers_to_helicopter(&mut self) {
        let Some((position, _, _)) = self.police.helicopter_motion() else { return };
        let point = glam::Vec2::new(position.x, position.z);
        let hit = self.breaker_areas.iter().any(|area| {
            area.duration > 0.0
                && blackbox_pursuit::breaker::inside(glam::Vec2::new(area.position.x, area.position.z), area.radius, point)
        });
        if hit {
            log::info!("pursuit breaker: the helicopter is destroyed");
            self.police.helicopter_destroyed();
        }
    }
}

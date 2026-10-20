//! The chase as the police radio reads it: the cops (sight, distance, formation slot, health), the helicopter, the
//! roadblock, the player's heading and road, and how the pursuit stands. Child of `police`, so it reads the police and
//! the traffic directly; the machine that uses it is in `audio/chase/radio`.
//!
//! What the world cannot tell is approximated or left out, and said so: a cop's sight is the world's sight test applied
//! to that cop's range and cone (no second ray cast), "in view" is that sight within 150 m (the world has no camera
//! test), and a cop is "in formation" when the formation gives it a slot.

use blackbox_pursuit::CooldownStatus;
use blackbox_pursuit::formation::slots::assign;
use blackbox_pursuit::sight;
use blackbox_roads::{SegmentFilter, closest_segment};
use glam::{Vec2, Vec3};

use super::*;
use crate::audio::{CopFeed, Formation as FeedFormation, HeliFeed, PlayerFeed, RoadblockFeed, SpeechFeed, StatusFeed};
use crate::scenes::world::ai::car::Role;

/// Metres within which a cop that sees the player also counts as on the player's screen (an approximation).
const IN_VIEW_RANGE: f32 = 150.0;
/// Metres from a formation slot at which a cop counts as in position (the same figure as the busted timer's gap).
const IN_POSITION: f32 = 4.0;
/// The helicopter's id in the feed (the cars use their render ids, which count up from 1).
const HELI_ID: u32 = u32::MAX;
/// The id of a roadblock: the roadblock's cars carry no shared id, so the nearest car's key stands for it.
const NO_ROADBLOCK: u32 = 0;

fn feed_formation(formation: Option<Formation>) -> FeedFormation {
    match formation {
        Some(Formation::Pit) => FeedFormation::Pit,
        Some(Formation::BoxIn) => FeedFormation::BoxIn,
        Some(Formation::RollingBlock) => FeedFormation::RollingBlock,
        Some(Formation::Herd) => FeedFormation::Herd,
        Some(Formation::StaggerFollow) => FeedFormation::StaggerFollow,
        Some(Formation::Follow) | None => FeedFormation::Follow,
    }
}

impl super::super::TrafficWorld {
    /// The feed with the speech's part filled in from the world.
    pub(super) fn with_speech_feed(&self, mut feed: crate::audio::PursuitFeed) -> crate::audio::PursuitFeed {
        feed.speech = self.speech_feed(feed.secs_to_evade);
        feed
    }

    /// Remembers the player's view of this frame for the radio.
    pub(super) fn note_speech_view(&mut self, player: &PlayerView) {
        self.speech_view = Some(*player);
    }

    fn speech_feed(&self, _secs_to_evade: f32) -> SpeechFeed {
        let mut out = SpeechFeed::default();
        let Some(view) = self.speech_view else { return out };
        let at = Vec3::from_array(space::to_physics(view.position));
        let forward = Vec3::from_array(space::to_physics(view.forward)).normalize_or_zero();
        out.player = PlayerFeed {
            position: at.to_array(),
            velocity_dir: match view.speed.abs() > 0.5 {
                true => (forward * view.speed.signum()).to_array(),
                false => [0.0; 3],
            },
            forward: forward.to_array(),
            road: self.road_speech_id(at, forward),
            car_type: 0,
            car_colour: 0,
        };
        out.status = StatusFeed {
            cooling_down: self.police.cooldown_status() == CooldownStatus::CoolDown,
            formation: feed_formation(self.police.last_formation()),
            collapse_active: false,
            finisher_active: self.police.finisher_running(),
            evade_level: self.police.evade_level(),
            busted: self.police.busted_meter(),
            infractions: *self.police.infractions(),
        };
        self.list_cops(&mut out, at, forward);
        out.heli = self.heli_feed(at);
        out
    }

    /// The speech id of the road nearest to `at` (0 for an unnamed road), `None` off every road.
    fn road_speech_id(&self, at: Vec3, heading: Vec3) -> Option<u16> {
        let near = closest_segment(&self.network, &self.index, at, heading, 0.0, SegmentFilter::default())?;
        let road = self.network.segment(near.segment).road?;
        Some(self.network.roads.get(usize::from(road))?.speech_id)
    }

    /// Every cop car of the chase and the roadblock's cars, wrecked ones included for a while.
    fn list_cops(&self, out: &mut SpeechFeed, at: Vec3, forward: Vec3) {
        let level = self.police.level().unwrap_or(1);
        let chasers: Vec<usize> = (0..self.cars.len()).filter(|&i| self.cars[i].is_cop()).collect();
        let positions: Vec<Vec2> = chasers
            .iter()
            .map(|&i| {
                let p = self.cars[i].physics_position();
                Vec2::new(p.x, p.z)
            })
            .collect();
        let slots = match self.police.last_formation() {
            Some(formation) => assign(formation, &positions, Vec2::new(at.x, at.z), Vec2::new(forward.x, forward.z)),
            None => vec![None; positions.len()],
        };
        let right = Vec2::new(forward.z, -forward.x);
        let ahead = Vec2::new(forward.x, forward.z);
        for (i, car) in self.cars.iter().enumerate() {
            let listed = matches!(car.role, Role::Cop(_)) || car.is_roadblock();
            if !listed {
                continue;
            }
            let position = car.physics_position();
            let to_cop = position - at;
            let distance = to_cop.length();
            let facing = car.physics_forward().dot((-to_cop).try_normalize().unwrap_or(Vec3::ZERO));
            let sees = self.seen_by_cops && !car.is_wrecked() && !sight::out_of_range(level, distance, facing);
            let slot = chasers.iter().position(|&c| c == i).and_then(|c| slots.get(c).copied().flatten());
            let offset = slot.map_or([0.0; 2], |aim| {
                let d = aim - Vec2::new(at.x, at.z);
                [d.dot(right), d.dot(ahead)]
            });
            let sample = car.cop_health_sample();
            let damage = sample.map_or(0.0, |s| s.damage);
            out.push_cop(CopFeed {
                id: sample.map_or_else(|| car.render_key(), |s| s.key),
                active: !car.is_wrecked(),
                destroyed: car.is_wrecked(),
                health: (1.0 - damage).clamp(0.0, 1.0),
                distance,
                speed_mph: car.speed().abs() / super::super::traffic::MPH,
                position: position.to_array(),
                los: sees,
                in_view: sees && distance < IN_VIEW_RANGE,
                ahead: horizontal(to_cop).normalize_or_zero().dot(horizontal(forward).normalize_or_zero()) > 0.0,
                in_formation: slot.is_some(),
                in_position: slot.is_some_and(|aim| Vec2::new(position.x, position.z).distance(aim) <= IN_POSITION),
                offset,
                vehicle: crate::audio::radio_tables::vehicle_of(&car.name),
                road: self.road_speech_id(position, car.physics_forward()).unwrap_or(0),
                roadblock: car.is_roadblock(),
            });
        }
        out.roadblock = self.roadblock_feed(out, at);
    }

    /// The roadblock, when its cars stand: the nearest of its cars gives it an identity that holds while it stands.
    fn roadblock_feed(&self, out: &SpeechFeed, at: Vec3) -> Option<RoadblockFeed> {
        if !self.police.roadblock_standing() {
            return None;
        }
        let cars: Vec<&CopFeed> = out.cops().iter().filter(|c| c.roadblock).collect();
        let id = cars.iter().map(|c| c.id).min().unwrap_or(NO_ROADBLOCK + 1);
        let nearest = cars.iter().map(|c| Vec3::from_array(c.position).distance(at)).fold(f32::INFINITY, f32::min);
        Some(RoadblockFeed {
            id,
            visual: cars.iter().any(|c| c.los || c.in_view),
            spike_strips: self.spikes.len().min(255) as u8,
            distance: nearest,
        })
    }

    /// The helicopter, while it is in the chase.
    fn heli_feed(&self, at: Vec3) -> Option<HeliFeed> {
        let heli = self.police.heli_view()?;
        Some(HeliFeed {
            id: HELI_ID,
            active: !heli.leaving,
            los: heli.sees,
            distance: heli.position.distance(at),
            fuel_secs: heli.fuel,
        })
    }
}

/// The helicopter as the radio sees it.
pub(super) struct HeliView {
    pub position: Vec3,
    pub sees: bool,
    pub leaving: bool,
    pub fuel: f32,
}

impl Police {
    /// The helicopter's state for the radio, `None` without one.
    pub(super) fn heli_view(&self) -> Option<HeliView> {
        let heli = self.heli.as_ref()?;
        Some(HeliView {
            position: heli.body.position,
            sees: heli.sight.visible(),
            leaving: matches!(heli.mode, blackbox_pursuit::helicopter::Mode::Exit(_)),
            fuel: heli.fuel.max(0.0),
        })
    }
}

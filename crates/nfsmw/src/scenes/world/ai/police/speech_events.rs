//! The world's events for the chase radio: what the player's car does (a crash, a jump, a roll, the pedals), where it
//! drives (a tunnel, the way the road runs) and what happens around it (a cop passed, a roadblock dodged, backup
//! called). The facts are noted here as they happen and ride in the feed's [`EventLog`]; the radio decides what is
//! worth a remark (`audio/chase/radio/world_notices.rs`). Spec: `docs/specs/ai-pursuit-speech.md` section 7.
//!
//! The recorders for cars hitting cars are in `speech_hits.rs`.

use blackbox_collision::CollisionWorld;
use blackbox_roads::{SegmentFilter, TrackZone, centre_line, closest_segment, to_zone_space};
use glam::Vec3;

use crate::audio::{Crash, EventLog, WorldEvent};
use crate::scenes::world::ai::TrafficWorld;
use crate::scenes::world::drive::{CarSim, StepMemo};

/// A wall or prop hit that took less than this much speed (metres per second) cannot be a crash or a remark: it is not
/// logged, so a scrape does not fill the log.
const MIN_HIT: f32 = 0.5;
/// The tunnel-like zones: tunnel, overpass, small overpass, garage and dynamic (`spatial-parameter-maps.md`).
const ENCLOSURES: [u32; 5] = [3, 4, 5, 7, 10];
/// A cop passed within the whoosh distance faster than this relative speed (m/s) has been "blown by"; the same figures
/// as the cop car's drive-by sound (12 m, 15 m/s) **[decomp]**.
const BLOW_BY_SPEED: f32 = 15.0;
/// The same cop is not announced again within this many seconds (the original repeats the message every frame of the
/// pass, and the radio's queue drops the repeats).
const BLOW_BY_PAUSE: f32 = 1.0;

/// The player's pedals and attitude as the last collision step left them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Driving {
    pub brake: f32,
    pub handbrake: f32,
    pub up_y: f32,
}

impl Default for Driving {
    fn default() -> Self {
        Self { brake: 0.0, handbrake: 0.0, up_y: 1.0 }
    }
}

/// The time the car spent in the air: it ends with the landing.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Air {
    secs: f32,
    peak: f32,
    airborne: bool,
}

impl Air {
    /// One step. `altitude` is the car's height above the ground now. Returns the jump (its peak height and its length)
    /// on the step the wheels touch down again.
    pub fn step(&mut self, grounded: bool, altitude: f32, dt: f32) -> Option<(f32, f32)> {
        if !grounded {
            self.airborne = true;
            self.secs += dt;
            self.peak = self.peak.max(altitude);
            return None;
        }
        let jump = self.airborne.then_some((self.peak, self.secs));
        *self = Self::default();
        jump
    }

    pub fn in_air(&self) -> bool {
        self.airborne
    }
}

/// Which way a road runs for the car on it: the road's tangent turned to the way the car went, so that it keeps its sense
/// while the car swings round (a U-turn). The previous sense holds while the road still runs about the same line; on a
/// road across the old one the car's heading chooses.
pub(super) fn orient_road(tangent: Vec3, previous: Option<Vec3>, heading: Vec3) -> Vec3 {
    let reference = match previous {
        Some(prev) if prev.dot(tangent).abs() >= 0.5 => prev,
        _ => heading,
    };
    match reference.dot(tangent) < 0.0 {
        true => -tangent,
        false => tangent,
    }
}

/// What the police radio keeps watch of between frames.
#[derive(Default)]
pub(in crate::scenes::world::ai) struct SpeechWatch {
    /// The events of the last moments, for the feed.
    pub log: EventLog,
    pub(super) driving: Driving,
    pub(super) road_forward: Option<Vec3>,
    pub(super) air: Air,
    /// The ground under the car while it is in the air, and the height it took off from.
    ground_y: Option<f32>,
    takeoff_y: f32,
    /// The player is inside a tunnel or another enclosure.
    inside: bool,
    zones: Vec<TrackZone>,
    /// The cops that were announced as passed, with the time.
    passed: Vec<(u32, f32)>,
    clock: f32,
}

impl SpeechWatch {
    pub fn set_zones(&mut self, zones: Vec<TrackZone>) {
        self.zones = zones.into_iter().filter(|z| ENCLOSURES.contains(&z.kind)).collect();
    }

    /// Whether physics-space `at` is inside a tunnel, an overpass, a garage or a dynamic enclosure (the first zone of
    /// each kind that holds the point counts; a dynamic zone with visit info 1 does not).
    fn enclosed(&self, at: Vec3) -> bool {
        let point = to_zone_space(at);
        ENCLOSURES.iter().any(|&kind| {
            let zone = self
                .zones
                .iter()
                .find(|z| z.kind == kind && z.contains(point) && (z.elevation == 0.0 || z.elevation >= at.y));
            zone.is_some_and(|z| !(kind == 10 && z.visit_info == 1))
        })
    }

    pub(super) fn push(&mut self, event: WorldEvent) {
        self.log.push(event);
    }
}

impl TrafficWorld {
    /// Notes a fact for the chase radio.
    pub(in crate::scenes::world::ai) fn note_speech(&mut self, event: WorldEvent) {
        self.speech_watch.push(event);
    }

    /// Gives the radio the track's enclosure zones (read once when the track opens).
    pub(in crate::scenes::world::ai) fn set_speech_zones(&mut self, zones: Vec<TrackZone>) {
        self.speech_watch.set_zones(zones);
    }

    /// Once per police tick: where the road runs under the player and whether the player entered or left a tunnel.
    pub(super) fn watch_view(&mut self, at: Vec3, heading: Vec3) {
        self.speech_watch.road_forward = self.road_sense(at, heading);
        let inside = self.speech_watch.enclosed(at);
        if inside == self.speech_watch.inside {
            return;
        }
        self.speech_watch.inside = inside;
        self.note_speech(WorldEvent::Tunnel { inside });
    }

    fn road_sense(&self, at: Vec3, heading: Vec3) -> Option<Vec3> {
        let near = closest_segment(&self.network, &self.index, at, heading, 0.0, SegmentFilter::default())?;
        let tangent = centre_line(&self.network, near.segment, 1).tangent(near.t).try_normalize()?;
        Some(orient_road(tangent, self.speech_watch.road_forward, heading))
    }

    /// Before the cars step: the ground under the player while it is in the air, for the height of a jump.
    pub(in crate::scenes::world::ai) fn watch_ground(&mut self, player: Option<Vec3>, collision: &CollisionWorld) {
        let watch = &mut self.speech_watch;
        watch.ground_y = None;
        let Some(at) = player.filter(|_| watch.air.in_air()) else { return };
        watch.ground_y = collision.ground(at.x, at.z, at.y + 1.0, at.y - 80.0).map(|hit| hit.point[1]);
    }

    /// Once per collision step, after the player's car moved: its pedals and attitude, what it ran into, and its jumps.
    pub(in crate::scenes::world::ai) fn watch_player(&mut self, player: &CarSim) {
        let memo = player.step_memo();
        self.speech_watch.driving = Driving { brake: memo.brake, handbrake: memo.handbrake, up_y: player.up_dot() };
        self.note_player_hit(memo);
        self.note_roadblock_objects(memo);
        self.watch_air(player);
    }

    fn note_player_hit(&mut self, memo: &StepMemo) {
        let Some(hit) = memo.hit.filter(|h| h.delta_v >= MIN_HIT) else { return };
        let crash = Crash {
            delta_v: hit.delta_v,
            closing: hit.closing,
            speed_before: memo.speed_before,
            speed_after: memo.speed_after,
        };
        use crate::scenes::world::drive::walls::ContactKind;
        let event = match hit.kind {
            ContactKind::Barrier | ContactKind::Face => WorldEvent::PlayerHitWorld(crash),
            ContactKind::PropRigid | ContactKind::PropMovable => WorldEvent::PlayerHitProp(crash),
        };
        self.note_speech(event);
    }

    /// The player ran into one of the roadblock's sawhorses.
    fn note_roadblock_objects(&mut self, memo: &StepMemo) {
        if !memo.props.iter().any(|&id| self.roadblocks.owns_sawhorse(id)) {
            return;
        }
        self.note_speech(WorldEvent::RoadblockHit(blackbox_pursuit::speech::RoadblockHit::Object));
    }

    fn watch_air(&mut self, player: &CarSim) {
        let body = player.body();
        let grounded = player.telemetry().wheels_on_ground > 0;
        let watch = &mut self.speech_watch;
        if grounded {
            watch.takeoff_y = body.position.y;
        }
        let altitude = match watch.ground_y {
            Some(ground) => body.position.y - ground - player.half_extents().y,
            None => body.position.y - watch.takeoff_y,
        };
        let Some((altitude, secs)) = watch.air.step(grounded, altitude, self.step_delta) else { return };
        self.note_speech(WorldEvent::Airborne { altitude, secs });
    }

    /// A cop passed the player close and fast (`BlewByCop`).
    pub(in crate::scenes::world::ai) fn note_pass(&mut self, owner: usize, relative_speed: f32) {
        let car = &self.cars[owner];
        if relative_speed < BLOW_BY_SPEED || !car.is_cop_class() {
            return;
        }
        let cop = car.speech_id();
        let now = self.speech_watch.clock;
        let watch = &mut self.speech_watch;
        watch.passed.retain(|&(_, at)| now - at < BLOW_BY_PAUSE);
        if watch.passed.iter().any(|&(id, _)| id == cop) {
            return;
        }
        watch.passed.push((cop, now));
        self.note_speech(WorldEvent::BlewByCop { cop });
    }

    /// The radio's clock, in simulated seconds, for its pauses.
    pub(in crate::scenes::world::ai) fn tick_speech_watch(&mut self, dt: f32) {
        self.speech_watch.clock += dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_jump_is_told_when_the_wheels_come_down() {
        let mut air = Air::default();
        assert_eq!(air.step(true, 0.0, 0.016), None);
        assert_eq!(air.step(false, 0.4, 0.5), None);
        assert!(air.in_air());
        assert_eq!(air.step(false, 1.6, 0.5), None);
        assert_eq!(air.step(false, 0.8, 0.5), None);
        let (peak, secs) = air.step(true, 0.0, 0.016).expect("the landing");
        assert_eq!(peak, 1.6);
        assert!((secs - 1.5).abs() < 1e-5);
        assert_eq!(air.step(true, 0.0, 0.016), None, "told once");
        assert!(!air.in_air());
    }

    #[test]
    fn the_road_keeps_its_sense_while_the_car_turns_round() {
        let north = Vec3::Z;
        let along = orient_road(north, None, Vec3::Z);
        assert_eq!(along, north);
        // A road drawn the other way round is turned to the car's way.
        assert_eq!(orient_road(-north, None, Vec3::Z), north);
        // The car swings round: the road keeps the sense it had.
        assert_eq!(orient_road(north, Some(north), -Vec3::Z), north);
        assert_eq!(orient_road(-north, Some(north), -Vec3::Z), north);
    }

    #[test]
    fn a_road_across_the_old_one_takes_the_cars_heading() {
        let turned = orient_road(Vec3::X, Some(Vec3::Z), -Vec3::X);
        assert_eq!(turned, -Vec3::X);
    }

    fn zone(kind: u32, elevation: f32, visit_info: i16) -> TrackZone {
        let corner = |x, y| glam::Vec2::new(x, y);
        TrackZone {
            kind,
            position: glam::Vec2::ZERO,
            direction: glam::Vec2::X,
            elevation,
            visit_info,
            data: [0; 4],
            bbox_min: glam::Vec2::splat(-10.0),
            bbox_max: glam::Vec2::splat(10.0),
            polygon: vec![corner(-10.0, -10.0), corner(10.0, -10.0), corner(10.0, 10.0), corner(-10.0, 10.0)],
        }
    }

    #[test]
    fn only_the_enclosure_kinds_are_kept() {
        let mut watch = SpeechWatch::default();
        watch.set_zones(vec![zone(3, 0.0, 0), zone(8, 0.0, 0), zone(10, 0.0, 0)]);
        assert_eq!(watch.zones.len(), 2, "a hiding spot is no enclosure");
    }

    #[test]
    fn a_tunnel_encloses_what_is_inside_its_outline() {
        let mut watch = SpeechWatch::default();
        watch.set_zones(vec![zone(3, 0.0, 0)]);
        assert!(watch.enclosed(Vec3::ZERO));
        assert!(!watch.enclosed(Vec3::new(500.0, 0.0, 500.0)));
    }

    #[test]
    fn an_overpass_with_an_elevation_encloses_only_what_is_under_it() {
        let mut watch = SpeechWatch::default();
        watch.set_zones(vec![zone(4, 6.0, 0)]);
        assert!(watch.enclosed(Vec3::new(0.0, 2.0, 0.0)), "under the deck");
        assert!(!watch.enclosed(Vec3::new(0.0, 9.0, 0.0)), "on top of it");
    }

    #[test]
    fn a_dynamic_zone_that_was_visited_encloses_nothing() {
        let mut watch = SpeechWatch::default();
        watch.set_zones(vec![zone(10, 0.0, 1)]);
        assert!(!watch.enclosed(Vec3::ZERO));
        watch.set_zones(vec![zone(10, 0.0, 0)]);
        assert!(watch.enclosed(Vec3::ZERO));
    }
}

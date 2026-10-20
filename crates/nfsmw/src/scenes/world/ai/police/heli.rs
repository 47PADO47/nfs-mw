//! The police helicopter in the chase: when it is allowed, how it is thought each frame and when it leaves. The model and
//! its rules are in `blackbox_pursuit::helicopter`; its tuning per heat and race level is in `settings::helicopter`. The
//! cop manager's side (the helicopter's turn among the wave's cop types and its spawn point) is `ai/air.rs`. Child of
//! `police`, so it reads the chase's state directly. Spec: `docs/specs/ai-helicopter.md` section 2.

use blackbox_pursuit::CooldownStatus;
use blackbox_pursuit::helicopter::control::{Flat, Heights};
use blackbox_pursuit::helicopter::{Helicopter, Mode, SearchInput, THINK_PERIOD, Target, Tuning};
use glam::Vec3;

use super::super::support;
use super::helpers::{SheetFloor, horizontal, to_body};
use super::{PlayerView, Police, Sensed};

/// Heat levels whose `pursuitsupport` has a skid-hit air entry (`SKID_HIT`, [verified]).
const SKID_LEVELS: [u32; 5] = [5, 6, 7, 9, 10];
/// `SearchModeHeliSpawnChance` by heat level 1 to 10, percent: the chance, when the cool-down meter first shows, that the
/// helicopter is forced as the next cop (spec section 2.1, [verified] in the install; zero elsewhere).
pub(in crate::scenes::world::ai) const SEARCH_SPAWN_CHANCE: [f32; 10] =
    [0.0, 0.0, 0.0, 2.0, 4.0, 5.0, 0.0, 0.0, 0.0, 0.0];
/// How high above the player it spawns when no cop road is found.
const SPAWN_HEIGHT: f32 = 20.0;
/// The race table is not used yet: no race code calls the police, so the chase always reads the heat table.
const RACING: bool = false;

/// Where the helicopter appears: physics space, the way it faces in radians in the x-z plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spawn {
    pub position: Vec3,
    pub heading: f32,
}

/// What the cop manager is told when it asks for the helicopter at a spawn opportunity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// Not now: it is not allowed yet, one is flying, or one is on its way.
    No,
    /// It may take its turn among the wave's types.
    Allowed,
    /// The search-mode roll forced it as the next cop.
    Forced,
}

/// The helicopter's spawn plan (`mDoTestForHeliSearch`, `mForceHeliSpawnNext`).
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Plan {
    /// The search-mode roll is still to be made (the cool-down meter appeared).
    search_armed: bool,
    /// The next cop is the helicopter whatever the wave says.
    forced: bool,
    /// A spawn the cop manager asked for; the next step places the helicopter there.
    queued: Option<Spawn>,
    /// The cool-down was running at the last step, to catch the meter appearing and going away.
    cooling: bool,
    /// The search-mode roll forced the helicopter, until it has left again.
    quadrant: bool,
    /// The pursuit's last known place of the player (physics space), which the search circles.
    last_known: Option<Vec3>,
}

/// What the helicopter is doing, as the speech and the HUD tell it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeliMode {
    Following,
    SkidHit,
    Searching,
    Leaving,
}

/// The helicopter's state for the speech and the HUD: whether it has the player in sight, its fuel and its health.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeliStatus {
    /// The last sight test saw the player (including the grace after a lost line).
    pub in_sight: bool,
    /// Seconds of fuel left; at zero it leaves.
    pub fuel_secs: f32,
    /// Fuel left as a fraction of the level's tank.
    pub fuel_fraction: f32,
    pub mode: HeliMode,
    /// 1 while it flies: the helicopter has no hit points (`HIT_POINTS` 0) and only a breaker zone destroys it.
    pub health: f32,
}

impl Police {
    /// Whether a helicopter is in the chase, flying or leaving.
    pub fn helicopter_present(&self) -> bool {
        self.heli.is_some()
    }

    /// Replaces the helicopter tuning (read from the file when the track opens).
    pub fn set_heli_settings(&mut self, settings: crate::settings::helicopter::HeliSettings) {
        self.heli_settings = settings;
    }

    /// The metres behind the player at which the helicopter appears at `level` (the mod's `SpawnDistance`).
    pub fn heli_spawn_distance(&self, level: u32) -> f32 {
        self.heli_settings.tuning(level, RACING).spawn_distance
    }

    /// The helicopter's state for the speech and the HUD, while it flies.
    pub fn helicopter_status(&self) -> Option<HeliStatus> {
        let heli = self.heli.as_ref()?;
        let mode = match heli.mode {
            Mode::Follow => HeliMode::Following,
            Mode::SkidApproach { .. } | Mode::SkidStrike { .. } => HeliMode::SkidHit,
            Mode::Search { .. } => HeliMode::Searching,
            Mode::Exit(_) => HeliMode::Leaving,
        };
        let tank = heli.tuning.fuel_time.max(f32::EPSILON);
        Some(HeliStatus {
            in_sight: heli.sight.visible(),
            fuel_secs: heli.fuel.max(0.0),
            fuel_fraction: (heli.fuel / tank).clamp(0.0, 1.0),
            mode,
            health: 1.0,
        })
    }

    /// The pursuit's last known place of the player, set by the world each frame; the search circles it.
    pub fn set_last_known(&mut self, at: Option<Vec3>) {
        self.heli_plan.last_known = at;
    }

    /// The helicopter's pose for the rotor sound: position, nose direction and velocity, physics space.
    pub fn helicopter_motion(&self) -> Option<(Vec3, Vec3, Vec3)> {
        let heli = self.heli.as_ref()?;
        Some((heli.body.position, heli.body.forward(), heli.body.velocity))
    }

    /// A pursuit breaker destroyed the helicopter: it is gone at once, and the next one waits the level's time
    /// (`UpdatePursuits` resets the helicopter timer when the vehicle leaves the pursuit, destroyed or not).
    pub fn helicopter_destroyed(&mut self) {
        if self.heli.take().is_none() {
            return;
        }
        self.heli_wait = self.heli_settings.respawn_secs(self.level().unwrap_or(1), RACING);
        self.heli_plan.quadrant = false;
    }

    /// The search-mode roll forced the helicopter and it has not left yet: this is the quadrant-search call's moment (the
    /// speech edge-detects it). Clears when the helicopter is gone.
    pub fn heli_search_call(&self) -> bool {
        self.heli_plan.quadrant
    }

    /// The cop request for the helicopter (`CopRequest`): it must be allowed (its timer spent: 10 s at the start of the chase,
    /// then the level's wait after one left; the support row's minimum delay passed; none flying or coming; the model
    /// loaded; the player not busted). The first time the question is asked after the cool-down meter appeared, the
    /// search-mode roll may force it: `roll` is uniform in `0..=1` and the chance is the level's percent.
    pub fn heli_ask(&mut self, level: u32, roll: f32) -> Ask {
        let allowed = self.heli_available
            && self.heli.is_none()
            && self.heli_plan.queued.is_none()
            && self.heli_wait <= 0.0
            && !self.is_busted()
            && support::minimum_delay(level) < self.pursuit_time;
        if !allowed {
            return Ask::No;
        }
        if self.heli_plan.search_armed {
            self.heli_plan.search_armed = false;
            let chance = SEARCH_SPAWN_CHANCE[(level as usize).clamp(1, 10) - 1];
            if chance > 0.0 && !self.is_perp_in_sight() && roll * 100.0 <= chance {
                self.heli_plan.forced = true;
                self.heli_plan.quadrant = true;
            }
        }
        match self.heli_plan.forced {
            true => Ask::Forced,
            false => Ask::Allowed,
        }
    }

    /// The cop manager picked the helicopter: it appears at `spawn` on the next step.
    pub fn queue_helicopter(&mut self, spawn: Spawn) {
        self.heli_plan.queued = Some(spawn);
    }

    /// Catches the cool-down meter appearing (arms the search-mode roll) and the chase coming back (disarms it).
    pub(super) fn note_heli_cooldown(&mut self) {
        let cooling = self.cooldown.status() == CooldownStatus::CoolDown;
        match (self.heli_plan.cooling, cooling) {
            (false, true) => self.heli_plan.search_armed = true,
            (true, false) => self.heli_plan.search_armed = false,
            _ => {}
        }
        self.heli_plan.cooling = cooling;
    }

    pub(super) fn think_helicopter(
        &mut self,
        dt: f32,
        pursuing: bool,
        player: &PlayerView,
        sensed: Sensed,
        clear: &dyn Fn(Vec3, Vec3) -> bool,
    ) {
        if self.heli.is_none() && pursuing && self.heli_plan.queued.is_some() {
            self.try_join(player);
        }
        let Some(mut heli) = self.heli.take() else { return };
        let from = heli.body.position;
        let line = |to: Vec3| clear(from, to + Vec3::Y);
        let body = to_body(player);
        let flat = Flat(body.position.y - 2.0);
        let sheet = SheetFloor(self.heli_sheet.as_ref());
        let heights: &dyn Heights = match self.heli_settings.heli_sheet && self.heli_sheet.is_some() {
            true => &sheet,
            false => &flat,
        };
        let target = Target { hidden: sensed.hidden_from_heli, in_tunnel: sensed.in_tunnel };
        self.heli_clock += dt;
        let perp_in_sight = self.is_perp_in_sight();
        let mut gone = false;
        while self.heli_clock >= THINK_PERIOD && !gone {
            self.heli_clock -= THINK_PERIOD;
            let search = SearchInput {
                perp_in_sight,
                last_known: self.heli_plan.last_known.unwrap_or(body.position),
                unit_random: sensed.unit_random,
            };
            gone = heli.think(THINK_PERIOD, &body, target, &line, heights, search);
        }
        if gone {
            self.heli_wait = self.heli_settings.respawn_secs(self.level().unwrap_or(1), RACING);
            self.heli_plan.quadrant = false;
            return;
        }
        if heli.too_far(&body) && !matches!(heli.mode, Mode::Exit(_)) {
            heli.leave();
        }
        self.heli = Some(heli);
    }

    /// Puts the helicopter in the chase: where the cop manager queued it, or straight behind the player when no spawn was
    /// queued. It spawns `spawn_distance` metres behind the player on the road (`SpawnPursuitHelicopter`).
    pub(super) fn try_join(&mut self, player: &PlayerView) {
        let Some(level) = self.level() else { return };
        if !self.heli_available || self.heli.is_some() {
            return;
        }
        let tuning = Tuning { skid_hit: SKID_LEVELS.contains(&level), ..self.heli_settings.tuning(level, RACING) };
        let body = to_body(player);
        let (position, heading) = match self.heli_plan.queued.take() {
            Some(spawn) => (spawn.position, spawn.heading),
            None => (
                body.position - horizontal(body.forward).normalize_or_zero() * tuning.spawn_distance
                    + Vec3::new(0.0, SPAWN_HEIGHT, 0.0),
                body.forward.z.atan2(body.forward.x),
            ),
        };
        self.heli = Some(Helicopter::spawn(position, heading, tuning));
        self.heli_plan.forced = false;
        self.on_helicopter();
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{sensed, view};
    use super::*;

    fn chase(level: u32) -> Police {
        let mut police = Police::default();
        police.begin(level);
        police
    }

    /// A chase that has lasted long enough, with the first wait spent.
    fn ready(level: u32) -> Police {
        let mut police = chase(level);
        police.heli_wait = 0.0;
        police.pursuit_time = 1000.0;
        police
    }

    #[test]
    fn the_helicopter_waits_for_its_timer_and_the_minimum_support_delay() {
        let mut police = chase(5);
        assert_eq!(police.heli_ask(5, 0.9), Ask::No, "the 10 s timer has not run out");
        police.heli_wait = 0.0;
        assert_eq!(police.heli_ask(5, 0.9), Ask::No, "heat 5 asks 35 s of chase first");
        police.pursuit_time = 36.0;
        assert_eq!(police.heli_ask(5, 0.9), Ask::Allowed);
    }

    #[test]
    fn one_helicopter_at_a_time_and_none_while_one_is_on_its_way() {
        let mut police = ready(5);
        police.queue_helicopter(Spawn { position: Vec3::ZERO, heading: 0.0 });
        assert_eq!(police.heli_ask(5, 0.9), Ask::No);
        police.try_join(&PlayerView {
            position: Vec3::ZERO,
            forward: Vec3::X,
            speed: 0.0,
            nitrous: 0.0,
            throttle: 0.0,
        });
        assert!(police.helicopter_present());
        assert_eq!(police.heli_ask(5, 0.9), Ask::No);
    }

    #[test]
    fn a_busted_player_is_not_chased_by_a_new_helicopter() {
        let mut police = ready(5);
        police.start_bust(0.0);
        assert_eq!(police.heli_ask(5, 0.9), Ask::No);
    }

    #[test]
    fn losing_the_cops_arms_the_search_roll_once_and_a_low_roll_forces_the_helicopter() {
        let mut police = ready(5);
        police.update(1.0, true, &view(10.0), sensed(true));
        for _ in 0..240 {
            police.update(0.05, true, &view(10.0), sensed(false));
        }
        assert_eq!(police.cooldown_status(), CooldownStatus::CoolDown);
        assert!(police.heli_plan.search_armed, "the meter appeared");
        assert_eq!(police.heli_ask(5, 0.03), Ask::Forced, "3 percent is under the 4 of heat 5");
        assert!(police.heli_search_call(), "the quadrant call is due");
        assert!(!police.heli_plan.search_armed, "the roll is spent");
    }

    #[test]
    fn the_chase_coming_back_disarms_the_search_roll() {
        let mut police = ready(5);
        police.update(1.0, true, &view(10.0), sensed(true));
        for _ in 0..240 {
            police.update(0.05, true, &view(10.0), sensed(false));
        }
        for _ in 0..80 {
            police.update(0.05, true, &view(10.0), sensed(true));
        }
        assert_eq!(police.cooldown_status(), CooldownStatus::InitialChase);
        assert!(!police.heli_plan.search_armed);
    }

    #[test]
    fn a_high_roll_leaves_the_helicopter_to_the_wave() {
        let mut police = ready(5);
        police.heli_plan.search_armed = true;
        assert_eq!(police.heli_ask(5, 0.5), Ask::Allowed);
        assert!(!police.heli_search_call());
        assert_eq!(police.heli_ask(5, 0.0), Ask::Allowed, "the roll is made only once per loss of contact");
    }

    #[test]
    fn heat_one_to_three_have_no_search_spawn() {
        let mut police = ready(3);
        police.heli_plan.search_armed = true;
        assert_eq!(police.heli_ask(3, 0.0), Ask::Allowed, "a zero chance never forces it");
    }

    #[test]
    fn a_breaker_destroys_the_helicopter_and_the_next_one_waits() {
        let mut police = ready(5);
        police.queue_helicopter(Spawn { position: Vec3::ZERO, heading: 0.0 });
        police.try_join(&PlayerView {
            position: Vec3::ZERO,
            forward: Vec3::X,
            speed: 0.0,
            nitrous: 0.0,
            throttle: 0.0,
        });
        police.helicopter_destroyed();
        assert!(!police.helicopter_present());
        assert_eq!(police.heli_wait, 180.0, "heat 5 waits 180 s");
        assert_eq!(police.heli_ask(5, 0.9), Ask::No);
    }

    #[test]
    fn the_status_tells_fuel_sight_and_mode_for_the_speech() {
        let mut police = ready(5);
        assert!(police.helicopter_status().is_none());
        police.queue_helicopter(Spawn { position: Vec3::ZERO, heading: 0.0 });
        police.try_join(&PlayerView {
            position: Vec3::ZERO,
            forward: Vec3::X,
            speed: 0.0,
            nitrous: 0.0,
            throttle: 0.0,
        });
        let status = police.helicopter_status().expect("flying");
        assert_eq!(status.mode, HeliMode::Following);
        assert_eq!(status.fuel_secs, 90.0, "heat 5's tank");
        assert_eq!(status.fuel_fraction, 1.0);
        assert_eq!(status.health, 1.0);
    }
}

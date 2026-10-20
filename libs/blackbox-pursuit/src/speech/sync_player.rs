//! The player as the speech tracks it: the speed and the stop timer, the heading held for three seconds, the heat jumps
//! and the free-roam 911 call (`SyncPlayers`, `CalcPlayerDirection` of the original).

use super::actors::Pick;
use super::cx::Cx;
use super::params::{Direction, Jurisdiction, PursuitType};
use super::state::{Focus, Heading, PursuitState};

/// The heat levels whose crossing is announced, highest last (the radio's reaction to the wanted level rising).
const HEAT_CUTOFFS: [f32; 4] = [2.0, 3.0, 4.0, 5.0];
/// Seconds a heading must hold before it is reported.
const HEADING_HOLD: f32 = 3.0;
/// Seconds of chase before a heat jump is announced.
const HEAT_JUMP_AFTER: f32 = 5.0;
/// Seconds of lockout left in which the scripted 911 call may happen.
const SCRIPTED_911_WINDOW: f32 = 20.0;

impl Cx<'_> {
    /// The direction of the smoothed velocity, once it has held for three seconds (or at once if `force`).
    fn calc_player_direction(&mut self, force: bool) -> Option<Direction> {
        let fw = self.core.smoothed_forward;
        let direction = match (fw.x.abs() > fw.z.abs(), fw.x > 0.0, fw.z > 0.0) {
            (true, true, _) => Direction::East,
            (true, false, _) => Direction::West,
            (false, _, true) => Direction::South,
            (false, _, false) => Direction::North,
        };
        if self.core.direction_tracking != Some(direction) {
            self.core.direction_tracking = Some(direction);
            self.core.t_direction = self.now;
        }
        match self.now - self.core.t_direction > HEADING_HOLD || force {
            true => Some(direction),
            false => None,
        }
    }

    pub fn sync_player(&mut self) {
        let player = self.snap.player;
        let road = player.road.unwrap_or(0);
        let direction = self.calc_player_direction(false);
        self.core.current[0] = Heading { direction, road };
        if self.heading_valid() {
            self.follow_heading(direction, road);
        }
        self.track_speed(player.speed_mph);
        self.check_heat_jump();
        self.free_roam_911();
        let unit = player.velocity_dir.normalize_or_zero();
        self.core.smoothed_forward = self.core.smoothed_forward * 0.9 + unit * 0.1;
    }

    /// The heading the cops report moves on when the player turns onto another road in sight.
    fn follow_heading(&mut self, direction: Option<Direction>, road: u16) {
        let road_changed = self.core.current[1].road != road;
        self.core.current[1] = self.core.current[0];
        let p = self.snap.pursuit;
        if self.core.last_known.direction == direction || !p.exists || !p.perp_in_sight || !road_changed {
            return;
        }
        if self.state_is(PursuitState::Active)
            && !self.focus_is(Focus::Pursuit)
            && let Some(cop) = self.roster.closest(true, true)
        {
            self.direction_change(cop);
        }
        self.core.last_known = Heading { direction, road };
    }

    fn track_speed(&mut self, speed: f32) {
        let flags = &mut self.core.flags;
        if speed < self.tune.min_speed_stopped && !flags.low_speed_timer {
            self.core.t_low_speed = self.now;
            flags.low_speed_timer = true;
            return;
        }
        if speed >= self.tune.min_speed_stopped {
            flags.low_speed_timer = false;
            self.core.t_low_speed = self.now;
        }
    }

    /// The heat rising across 2, 3, 4 or 5 after five seconds of chase: dispatch, a primary cop or the helicopter says
    /// so, and perhaps dispatch announces the jurisdiction shifting to the state police or the federal agents.
    fn check_heat_jump(&mut self) {
        let heat = self.snap.pursuit.heat;
        if self.focus_is(Focus::Pursuit) || self.core.duration <= HEAT_JUMP_AFTER || heat <= self.core.prev_heat {
            return;
        }
        let prev = self.core.prev_heat;
        let Some(level) = HEAT_CUTOFFS.iter().rposition(|&cut| prev < cut && heat >= cut) else { return };
        self.core.prev_heat = heat;
        if !matches!(self.core.state, PursuitState::Active | PursuitState::Searching) {
            return;
        }
        let reached = level as u8 + 2;
        if self.rng.coin() {
            self.disp_heat_jump(reached);
        } else {
            let cop = self.random_active(Pick::Primary, false);
            let heli = self.roster.actors.iter().find(|a| a.heli).map(|a| a.id);
            let speaker = match heli {
                Some(heli) if self.rng.coin() => Some(heli),
                _ => cop,
            };
            if let Some(speaker) = speaker {
                self.heat_jump(speaker, reached);
            }
        }
        if !self.rng.coin() {
            return;
        }
        if (3.0..4.0).contains(&heat) {
            self.disp_jurisdiction_shift(Jurisdiction::State);
            return;
        }
        if heat >= 5.0 {
            self.disp_jurisdiction_shift(Jurisdiction::Federal);
        }
    }

    /// With no pursuit, dispatch reports 911 when the damage cost or the hard traffic hits reach their thresholds (free
    /// roam), or in the last seconds of the cop lockout (scripted); then the call ends with the 911 timer set.
    fn free_roam_911(&mut self) {
        let level = self.snap.level;
        if !self.state_is(PursuitState::Inactive) || self.core.flags.disp911_active || !level.cops_enabled {
            return;
        }
        let scripted = level.lockout_remaining > 0.0
            && level.lockout_remaining < SCRIPTED_911_WINDOW
            && !level.pursuit_race
            && !level.roaming;
        let over_cost = level.cts_for_911 > 0 && self.core.cost_911 >= level.cts_for_911;
        let over_hits = level.hits_for_911 > 0 && self.core.traffic_hits_911 >= level.hits_for_911;
        let requested = (over_cost || over_hits) && level.roaming;
        if !(requested || scripted) || level.ddays {
            return;
        }
        let kind = match self.high_intensity() {
            true => PursuitType::PossibleWanted,
            false => PursuitType::GenericSpeeder,
        };
        self.disp_report_911(kind);
    }
}

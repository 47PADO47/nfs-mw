//! The police helicopter: its tuning, its flight model, its controller, its sight and its behaviours. The caller
//! holds one [`Tuning`] per heat level (the game's values by default) and one [`Helicopter`] while it flies.
//! Spec: `docs/specs/ai-helicopter.md`.

pub mod behaviour;
pub mod control;
pub mod flight;
pub mod gains;
pub mod sight;

use glam::Vec3;

use behaviour::{Exit, Player, horizontal_distance};
use control::{Goal, Heights};
use flight::{Chopper, Specs};
pub use gains::Gains;
use sight::{Look, Sight};

/// The settings of one heat level that change how the helicopter behaves. The defaults are the game's own values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tuning {
    /// Metres along the road behind the target where it spawns (`SpawnPursuitHelicopter`: 250). [decomp] pool;
    /// the +20 m height is pool (applied by the game layer).
    pub spawn_distance: f32,
    /// Seconds its fuel lasts after it spawns (`HeliFuelTime`).
    pub fuel_time: f32,
    /// Metres it sees to its front (`heliLOSdistance`: 251 or 301).
    pub line_of_sight: f32,
    /// Metres from the player at which it leaves (600; 375 for ground units). [unconfirmed]: the police code's value, not
    /// found in the pool (spec section 10).
    pub despawn_distance: f32,
    /// Top speed, m/s (`chopperspecs` `MAX_SPEED_MPS`: 100).
    pub speed_cap: f32,
    /// Whether it lunges in for skid hits (the level's `pursuitsupport` has a `SKID_HIT` air entry).
    pub skid_hit: bool,
    /// Seconds a skid hit's approach may run before its timer is spent (`SKID_TIMER`, 8; the mod's `AttackDuration`).
    pub attack_duration: f32,
    /// Speed (m/s) of the search flight (`SEARCH_SPEED`, 70; the mod's `SearchSpeed`).
    pub search_speed: f32,
    /// Speed (m/s) of the skid hit and the exit (`STRIKE_SPEED`, 100; the mod's `FlySpeed`).
    pub fly_speed: f32,
    /// Radius (m) within which it sees the player whatever its facing (`VISUAL_SPHERE`, 75; the mod's `AllAroundSight`).
    pub all_around_sight: f32,
    /// Beyond `all_around_sight` it sees only ahead (the mod's `FrontSightOnly`). Off, the whole line-of-sight range.
    pub front_sight_only: bool,
    /// Whether a hiding spot hides the player from it (the mod's `HidingSpots`). Off, the spots are ignored.
    pub hiding_spots: bool,
    /// Metres the search spreads around the last known place: the level's `SearchModeRoadblockRadius` (spec 6.1: the search
    /// reads the same field as the roadblock request; it is 1000 m only at heat 1 and 2).
    pub search_radius: f32,
    /// The follow, brake and flight numbers a tuning file may change.
    pub gains: Gains,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            spawn_distance: 250.0,
            fuel_time: 60.0,
            line_of_sight: 251.0,
            despawn_distance: 600.0,
            speed_cap: 100.0,
            skid_hit: false,
            attack_duration: behaviour::SKID_TIMER,
            search_speed: behaviour::SEARCH_SPEED,
            fly_speed: behaviour::STRIKE_SPEED,
            all_around_sight: sight::VISUAL_SPHERE,
            front_sight_only: true,
            hiding_spots: true,
            search_radius: 1000.0,
            gains: Gains::default(),
        }
    }
}

/// The seconds between two thinks: the helicopter thinks every 0.125 s. [decomp] pool and.
pub const THINK_PERIOD: f32 = 0.125;

/// What the helicopter is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Follow,
    /// The skid hit is coming in from `side` (+1 or -1 of the player's right).
    SkidApproach {
        side: f32,
    },
    SkidStrike {
        side: f32,
    },
    /// The player is out of sight: fly to `point`.
    Search {
        point: Vec3,
    },
    Exit(Exit),
}

/// The player as the helicopter's sight sees it: where it is and the flags the world sets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub hidden: bool,
    pub in_tunnel: bool,
}

/// What the helicopter takes from the pursuit: whether the pursuit has the player in sight (any cop within the grace, this
/// helicopter's own sight included), the last known place, and a random number for the search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchInput {
    /// The pursuit says the player is in sight (`IsPerpInSight`). Out of sight the helicopter searches, whatever its own
    /// line to the player is.
    pub perp_in_sight: bool,
    /// The place the player was last seen (`GetLastKnownLocation`), which the search circles.
    pub last_known: Vec3,
    /// A uniform random number in `0..=1` picking the next search distance.
    pub unit_random: f32,
}

/// A helicopter in flight: the body, the sight, the fuel, the skid timer and the mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Helicopter {
    pub body: Chopper,
    pub mode: Mode,
    pub sight: Sight,
    /// Seconds of fuel left.
    pub fuel: f32,
    /// Above zero while a skid hit runs; below zero the skid cooldown counts down.
    pub skid_timer: f32,
    pub tuning: Tuning,
    /// Where the search is on its circle, in turns; it advances an eighth of a turn at each new search point.
    pub search_angle: f32,
}

impl Helicopter {
    /// A helicopter at `position`, facing `heading`, with its fuel full.
    pub fn spawn(position: Vec3, heading: f32, tuning: Tuning) -> Self {
        Self {
            body: Chopper::new(position, heading),
            mode: Mode::Follow,
            sight: Sight::default(),
            fuel: tuning.fuel_time,
            skid_timer: 0.0,
            tuning,
            search_angle: 0.0,
        }
    }

    /// Whether it is far enough from the player to be removed from the pursuit.
    pub fn too_far(&self, player: &Player) -> bool {
        horizontal_distance(self.body.position, player.position) > self.tuning.despawn_distance
    }

    /// Starts leaving: climbs away from the player and flies out.
    pub fn leave(&mut self) {
        self.mode = Mode::Exit(Exit::start(self.body.position));
    }

    /// One think of `THINK_PERIOD` seconds. `in_pursuit` is false when the chase has ended, which sends it away. `clear`
    /// tests the line to the target. Returns `true` once an exit is finished and the helicopter should be removed.
    pub fn think(
        &mut self,
        dt: f32,
        player: &Player,
        target: Target,
        clear: &dyn Fn(Vec3) -> bool,
        heights: &dyn Heights,
        search: SearchInput,
    ) -> bool {
        self.fuel -= THINK_PERIOD;
        self.skid_timer -= THINK_PERIOD;
        let specs = Specs::with_gains(self.tuning.speed_cap, &self.tuning.gains);
        if let Mode::Exit(mut exit) = self.mode {
            let goal = exit.think(self.body.position, self.body.velocity, player, self.tuning.fly_speed);
            self.mode = Mode::Exit(exit);
            self.step(dt, goal, heights, &specs);
            return exit.finished(self.body.position, player);
        }
        if self.fuel <= 0.0 {
            self.leave();
            return false;
        }
        let look = Look {
            heli: self.body.position,
            forward: self.body.forward(),
            target: player.position,
            range: self.tuning.line_of_sight,
            sphere: self.tuning.all_around_sight,
            front_only: self.tuning.front_sight_only,
            hidden: target.hidden && self.tuning.hiding_spots,
            in_tunnel: target.in_tunnel,
        };
        // Its own sight feeds the pursuit's (the caller reads `sight.visible()`); what it does follows the pursuit's.
        self.sight.can_see(&look, || clear(player.position));
        let goal = match search.perp_in_sight {
            true => self.pursue(player),
            false => self.search(heights, search),
        };
        self.step(dt, goal, heights, &specs);
        false
    }

    /// The goal while the player is in sight: follow, or a skid hit (its modes switch by the skid rules).
    fn pursue(&mut self, player: &Player) -> Goal {
        let heli = self.body.position;
        self.update_skid(heli, player);
        let fly = self.tuning.fly_speed;
        match self.mode {
            Mode::SkidApproach { side } => behaviour::skid_approach(heli, player, side, fly),
            Mode::SkidStrike { side } => behaviour::skid_strike(player, side, fly),
            Mode::Follow | Mode::Search { .. } | Mode::Exit(_) => {
                self.mode = Mode::Follow;
                let skid_ready = self.tuning.skid_hit && self.skid_timer < -5.0;
                behaviour::follow(heli, player, self.skid_timer, skid_ready, &self.tuning.gains)
            }
        }
    }

    /// Moves between follow, approach and strike by the skid rules of the spec.
    fn update_skid(&mut self, heli: Vec3, player: &Player) {
        match self.mode {
            Mode::SkidApproach { side } if behaviour::skid_aborts(heli, player) => {
                self.abort_skid(side);
            }
            Mode::SkidApproach { side } if behaviour::skid_strikes(heli, player, side) => {
                self.skid_timer = self.skid_timer.max(2.0);
                self.mode = Mode::SkidStrike { side };
            }
            Mode::SkidStrike { side } if behaviour::skid_aborts(heli, player) => {
                self.abort_skid(side);
            }
            Mode::Follow
                if self.tuning.skid_hit && self.skid_timer < -5.0 && behaviour::skid_can_start(heli, player) =>
            {
                self.skid_timer = self.tuning.attack_duration;
                self.mode = Mode::SkidApproach { side: behaviour::skid_side(heli, player) };
            }
            _ => {}
        }
    }

    fn abort_skid(&mut self, _side: f32) {
        self.skid_timer = 0.0;
        self.mode = Mode::Follow;
    }

    /// The goal while the pursuit has lost the player: fly to a point around the last known place, and choose the next one
    /// when within 30 m of it (`SearchForPerp`, `SetNextPerpSearchDest`).
    fn search(&mut self, heights: &dyn Heights, search: SearchInput) -> Goal {
        let point = match self.mode {
            Mode::Search { point } if horizontal_distance(self.body.position, point) >= behaviour::SEARCH_ARRIVAL => {
                point
            }
            _ => {
                self.search_angle = (self.search_angle + behaviour::SEARCH_STEP_TURNS).fract();
                let floor = heights.elevation(search.last_known.x, search.last_known.z);
                behaviour::search_point(
                    search.last_known,
                    self.search_angle,
                    search.unit_random,
                    self.tuning.search_radius,
                    floor,
                )
            }
        };
        self.mode = Mode::Search { point };
        behaviour::search(point, self.body.velocity, self.tuning.search_speed)
    }

    fn step(&mut self, dt: f32, goal: Goal, heights: &dyn Heights, specs: &Specs) {
        let desired = control::desired(self.body.position, self.body.velocity, &goal, heights, &self.tuning.gains);
        self.body.velocity = desired.body_velocity.unwrap_or(self.body.velocity);
        self.body.step(dt, desired.velocity, goal.look_at - self.body.position, desired.max_decel, specs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use control::Flat;

    fn player() -> Player {
        Player { position: Vec3::ZERO, velocity: Vec3::new(30.0, 0.0, 0.0), forward: Vec3::X }
    }

    const OPEN: Target = Target { hidden: false, in_tunnel: false };
    const SEARCH: SearchInput = SearchInput { perp_in_sight: true, last_known: Vec3::ZERO, unit_random: 0.5 };
    const LOST: SearchInput = SearchInput { perp_in_sight: false, last_known: Vec3::ZERO, unit_random: 0.5 };

    fn clear(_: Vec3) -> bool {
        true
    }

    fn run(heli: &mut Helicopter, seconds: f32, target: Target, line: &dyn Fn(Vec3) -> bool) -> bool {
        run_in(heli, seconds, target, line, SEARCH)
    }

    fn run_in(
        heli: &mut Helicopter,
        seconds: f32,
        target: Target,
        line: &dyn Fn(Vec3) -> bool,
        input: SearchInput,
    ) -> bool {
        let steps = (seconds / THINK_PERIOD) as usize;
        let mut done = false;
        for _ in 0..steps {
            done |= heli.think(THINK_PERIOD, &player(), target, line, &Flat(0.0), input);
        }
        done
    }

    #[test]
    fn the_game_values_are_the_defaults() {
        let t = Tuning::default();
        assert_eq!(t.spawn_distance, 250.0);
        assert_eq!(t.despawn_distance, 600.0);
        assert_eq!(t.speed_cap, 100.0);
    }

    #[test]
    fn fuel_runs_out_and_the_helicopter_exits() {
        let tuning = Tuning { fuel_time: 1.0, ..Tuning::default() };
        let mut heli = Helicopter::spawn(Vec3::new(-100.0, 40.0, 0.0), 0.0, tuning);
        run(&mut heli, 2.0, OPEN, &clear);
        assert!(matches!(heli.mode, Mode::Exit(_)));
    }

    #[test]
    fn a_visible_player_is_followed_closely() {
        let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
        run(&mut heli, 10.0, OPEN, &clear);
        assert_eq!(heli.mode, Mode::Follow);
        let gap = horizontal_distance(heli.body.position, player().position);
        assert!(gap < 150.0, "keeps up with the car: {gap} m");
    }

    #[test]
    fn a_player_the_pursuit_has_lost_sends_it_to_search() {
        let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
        run_in(&mut heli, 0.5, OPEN, &clear, LOST);
        assert!(matches!(heli.mode, Mode::Search { .. }));
    }

    #[test]
    fn its_own_blocked_line_does_not_send_it_to_search_while_the_pursuit_has_the_player() {
        let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
        let blocked = |_: Vec3| false;
        run(&mut heli, 1.0, OPEN, &blocked);
        assert!(!heli.sight.visible(), "the helicopter itself has lost the line");
        assert_eq!(heli.mode, Mode::Follow, "but another cop sees the player, so it keeps flying to it");
    }

    #[test]
    fn the_search_keeps_its_point_until_it_is_within_thirty_metres() {
        let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
        run_in(&mut heli, THINK_PERIOD, OPEN, &clear, LOST);
        let Mode::Search { point } = heli.mode else { panic!("{:?}", heli.mode) };
        run_in(&mut heli, 2.0, OPEN, &clear, SearchInput { unit_random: 0.9, ..LOST });
        assert_eq!(heli.mode, Mode::Search { point }, "a new random number does not move the point");
        heli.body.position = Vec3::new(point.x, point.y, point.z + 10.0);
        run_in(&mut heli, THINK_PERIOD, OPEN, &clear, SearchInput { unit_random: 0.9, ..LOST });
        let Mode::Search { point: next } = heli.mode else { panic!("{:?}", heli.mode) };
        assert_ne!(next, point, "within 30 m the next point is chosen");
    }

    #[test]
    fn the_search_spreads_over_the_levels_radius_around_the_last_known_place() {
        let tuning = Tuning { search_radius: 10_000.0, ..Tuning::default() };
        let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, tuning);
        let input = SearchInput { last_known: Vec3::new(500.0, 0.0, 0.0), unit_random: 0.0, ..LOST };
        run_in(&mut heli, THINK_PERIOD, OPEN, &clear, input);
        let Mode::Search { point } = heli.mode else { panic!("{:?}", heli.mode) };
        let from_centre = horizontal_distance(point, input.last_known);
        assert!((from_centre - 2000.0).abs() < 1.0, "0.2 of the radius at the smallest roll: {from_centre}");
    }

    #[test]
    fn a_hidden_player_is_not_seen_once_latched() {
        let mut heli = Helicopter::spawn(Vec3::new(-150.0, 40.0, 0.0), 0.0, Tuning::default());
        let blocked = |_: Vec3| false;
        let hidden = Target { hidden: true, in_tunnel: false };
        run(&mut heli, 0.25, hidden, &blocked);
        assert!(!heli.sight.visible());
    }

    #[test]
    fn with_hiding_spots_off_a_hidden_player_stays_in_sight() {
        let tuning = Tuning { hiding_spots: false, ..Tuning::default() };
        let mut heli = Helicopter::spawn(Vec3::new(-100.0, 40.0, 0.0), 0.0, tuning);
        run(&mut heli, 0.25, OPEN, &clear);
        let blocked = |_: Vec3| false;
        let hidden = Target { hidden: true, in_tunnel: false };
        run(&mut heli, 0.25, hidden, &blocked);
        assert!(heli.sight.visible(), "the zone is ignored, so the player near its last seen place is still seen");
    }

    #[test]
    fn with_skid_hit_it_comes_in_beside_the_player_when_in_window() {
        let tuning = Tuning { skid_hit: true, ..Tuning::default() };
        let mut heli = Helicopter::spawn(Vec3::new(20.0, 8.0, 0.0), 0.0, tuning);
        heli.skid_timer = -10.0;
        run(&mut heli, THINK_PERIOD, OPEN, &clear);
        assert!(matches!(heli.mode, Mode::SkidApproach { .. }), "mode {:?}", heli.mode);
    }

    #[test]
    fn a_skid_is_not_started_during_cooldown() {
        let tuning = Tuning { skid_hit: true, ..Tuning::default() };
        let mut heli = Helicopter::spawn(Vec3::new(20.0, 8.0, 0.0), 0.0, tuning);
        heli.skid_timer = 0.0;
        run(&mut heli, THINK_PERIOD, OPEN, &clear);
        assert_eq!(heli.mode, Mode::Follow, "cooldown not over");
    }

    #[test]
    fn the_exit_reports_done_once_it_is_clear_of_the_car() {
        let mut heli = Helicopter::spawn(Vec3::new(-20.0, 10.0, 0.0), 0.0, Tuning::default());
        heli.leave();
        let done = run(&mut heli, 60.0, OPEN, &clear);
        assert!(done, "the exit finishes within a minute");
    }

    #[test]
    fn a_helicopter_far_back_is_removed() {
        let heli = Helicopter::spawn(Vec3::new(-700.0, 40.0, 0.0), 0.0, Tuning::default());
        assert!(heli.too_far(&player()));
    }
}

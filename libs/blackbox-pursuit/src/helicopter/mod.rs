//! The police helicopter: its tuning, its flight model, its controller, its sight and its behaviours. The caller
//! holds one [`Tuning`] per heat level (the game's values by default) and one [`Helicopter`] while it flies.
//! Spec: `docs/specs/ai-helicopter.md`.

pub mod behaviour;
pub mod camera;
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

/// Legacy standalone helper interval. The host's vehicle task runs once per eight simulation updates;
/// callers can use [`Helicopter::decide`] and [`Helicopter::integrate`] at separate rates.
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
    goal: Option<Goal>,
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
    /// The game camera, when the world gives one: the helicopter keeps out of a sphere in front of it (`camera.rs`).
    pub camera: Option<camera::Camera>,
}

impl Helicopter {
    /// A helicopter at `position`, facing `heading`, with its fuel full.
    pub fn spawn(position: Vec3, heading: f32, tuning: Tuning) -> Self {
        Self {
            goal: None,
            body: Chopper::new(position, heading),
            mode: Mode::Follow,
            sight: Sight::default(),
            fuel: tuning.fuel_time,
            skid_timer: 0.0,
            tuning,
            search_angle: 0.0,
            camera: None,
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

    /// One combined decision and flight step. `clear`
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
        let gone = self.decide(dt, player, target, clear, heights, search);
        self.integrate(dt, heights);
        gone || self.exit_finished(player)
    }

    /// Update behavior at its own cadence; retain the command for every intervening physics step.
    pub fn decide(
        &mut self,
        dt: f32,
        player: &Player,
        target: Target,
        clear: &dyn Fn(Vec3) -> bool,
        heights: &dyn Heights,
        search: SearchInput,
    ) -> bool {
        if !(dt >= 0.0 && dt.is_finite()) {
            return false;
        }
        self.fuel -= dt;
        self.skid_timer -= dt;
        if let Mode::Exit(mut exit) = self.mode {
            let goal = exit.think(self.body.position, self.body.velocity, player, self.tuning.fly_speed);
            self.mode = Mode::Exit(exit);
            self.goal = Some(goal);
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
        self.goal = Some(goal);
        false
    }

    /// Flight controller and rigid body run every simulation step, independently of AI decisions.
    pub fn integrate(&mut self, dt: f32, heights: &dyn Heights) {
        let Some(goal) = self.goal else { return };
        let specs = Specs::with_gains(self.tuning.speed_cap, &self.tuning.gains);
        self.step(dt, goal, heights, &specs);
    }

    pub fn exit_finished(&self, player: &Player) -> bool {
        matches!(self.mode, Mode::Exit(exit) if exit.finished(self.body.position, player))
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
        let desired =
            control::desired(self.body.position, self.body.velocity, &goal, heights, &self.tuning.gains, self.camera);
        self.body.velocity = desired.body_velocity.unwrap_or(self.body.velocity);
        self.body.step(dt, desired.velocity, goal.look_at - self.body.position, desired.max_decel, specs);
    }
}

#[cfg(test)]
mod tests;

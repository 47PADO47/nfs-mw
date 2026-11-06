//! The game moment cameras of the gameplay camera's director: the pursuit-start look at the cop, and the jump and crash
//! shots of the car, each with the slow motion it plays in. The pursuit breaker's cut is `moment.rs`. Spec:
//! `docs/specs/game-moment-camera.md`.
//!
//! The director keeps two clocks in real time: the cop clock and the car clock. A clock that runs out returns to the
//! gameplay camera, which then starts at a quarter speed and recovers over two seconds.

pub mod jump;
pub mod rules;
mod track_car;
mod track_cop;

use glam::Vec3;

use track_car::TrackCar;
pub use track_car::{Car, Framing};
pub use track_cop::Camera;
use track_cop::TrackCop;

/// What a shot asks of the world: line of sight, ground heights and the road. Render space.
pub trait Scene {
    /// Whether the line from `a` to `b` is clear of the world's faces and walls.
    fn clear(&self, a: Vec3, b: Vec3) -> bool;
    /// The ground height under `at`.
    fn ground(&self, at: Vec3) -> Option<f32>;
    /// The road `ahead` metres along from `at`, heading `forward`: the point on it and its left and right edges.
    fn road_ahead(&self, at: Vec3, forward: Vec3, ahead: f32) -> Option<(Vec3, [Vec3; 2])>;
}

/// A shot that may start this step, as the world found it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Start {
    /// A pursuit of the player began.
    Pursuit,
    /// The player's car took off and the jump test passed: the shot's length (s).
    Jump(f32),
    /// The player's car is about to hit a car; `roadblock` when it is a roadblock's.
    Crash { roadblock: bool },
}

/// The moment sounds (keys of the moment streams) a shot posts when it starts.
const SOUND_PURSUIT: &str = "spot_cop_camera";
const SOUND_JUMP: &str = "jump_cam";
const SOUND_CRASH: &str = "crash_cam";

/// What plays.
#[derive(Debug, Clone, PartialEq)]
enum Shot {
    /// The gameplay camera; its slow-down blend.
    Gameplay,
    Car(TrackCar),
    Cop(TrackCop),
}

/// The gameplay camera's slow motion: a blend that rises while a pursuit shot is about to start and falls after a shot.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Slowdown {
    blend: f32,
    /// Seconds the blend takes to fall from 1 (it rises four times as fast).
    rate: f32,
    rising: bool,
}

impl Slowdown {
    /// After a shot: full slow motion, back to normal over two seconds.
    fn returning() -> Self {
        Self { blend: 1.0, rate: 2.0, rising: false }
    }

    fn advance(&mut self, dt: f32) {
        let step = dt / self.rate.max(1e-3);
        self.blend = match self.rising {
            true => (self.blend + 4.0 * step).min(1.0),
            false => (self.blend - step).max(0.0),
        };
    }

    /// The share of real time the world runs at: a quarter at full blend, normal above 0.9.
    fn sim_speed(&self) -> f32 {
        let speed = (1.0 - self.blend) * 0.75 + 0.25;
        match self.blend > 0.001 && speed <= 0.9 {
            true => speed,
            false => 1.0,
        }
    }
}

/// The director's moment state.
#[derive(Debug, Clone)]
pub struct MomentCams {
    cop_clock: f32,
    car_clock: f32,
    /// The pursuit shot's lead-in: the gameplay camera slows down over this many seconds before the shot.
    lead_in: f32,
    returning: bool,
    shot: Shot,
    slowdown: Slowdown,
    /// The world speed the last shot set; the cop shot keeps the speed it found.
    speed: f32,
    /// The last car shot's eye: the next one stands apart from it.
    last_eye: Option<Vec3>,
    /// The next car shot is a roadblock crash.
    roadblock: bool,
    cut: bool,
    sounds: Vec<&'static str>,
    last: Option<String>,
}

impl Default for MomentCams {
    fn default() -> Self {
        Self {
            cop_clock: 0.0,
            car_clock: 0.0,
            lead_in: 0.0,
            returning: false,
            shot: Shot::Gameplay,
            slowdown: Slowdown::default(),
            speed: 1.0,
            last_eye: None,
            roadblock: false,
            cut: false,
            sounds: Vec::new(),
            last: None,
        }
    }
}

/// What the director knows each step.
pub struct Facts<'a> {
    /// The player's car.
    pub car: Car,
    pub pursued: bool,
    /// The gameplay camera this step.
    pub camera: Camera,
    /// The cops the pursuit shot may look at: chasing and seeing the player, within range, seen from the camera.
    pub cops: &'a [Vec3],
    pub scene: &'a dyn Scene,
}

impl MomentCams {
    /// Starts a shot. A pursuit shot starts its clock unless one runs; a jump replaces nothing but the gameplay camera;
    /// a crash only cuts away from the gameplay camera.
    pub fn start(&mut self, start: Start, facts: &Facts<'_>) {
        match start {
            Start::Pursuit if self.cop_clock <= 0.0 => {
                self.cop_clock = rules::COP_CLOCK;
                self.lead_in = 0.5;
                self.sounds.push(SOUND_PURSUIT);
                self.note("pursuit start");
            }
            Start::Jump(secs) if self.car_clock <= 0.0 => {
                self.car_clock = secs;
                self.sounds.push(SOUND_JUMP);
                self.note(&format!("jump, {secs:.1} s"));
                self.switch(true, facts);
            }
            Start::Crash { roadblock } if self.shot == Shot::Gameplay => {
                self.car_clock = rules::CRASH_SECS;
                self.roadblock |= roadblock;
                self.sounds.push(SOUND_CRASH);
                self.note(match roadblock {
                    true => "roadblock crash",
                    false => "crash",
                });
                self.switch(true, facts);
            }
            _ => {}
        }
    }

    /// One step of `dt` seconds of real time: the clocks, the choice of shot and the shot itself.
    pub fn update(&mut self, dt: f32, facts: &Facts<'_>) {
        if self.car_clock > 0.0 {
            self.car_clock -= dt;
        }
        if self.cop_clock > 0.0 {
            self.cop_clock -= dt;
        }
        let mut car = matches!(self.shot, Shot::Car(_));
        let mut cop = matches!(self.shot, Shot::Cop(_));
        if self.car_clock < 0.0 {
            self.car_clock = 0.0;
            self.returning = true;
            car = false;
        }
        if self.cop_clock < 0.0 {
            self.cop_clock = 0.0;
            self.returning = true;
            cop = false;
        }
        if self.cop_clock > 0.0 && self.cop_clock < rules::COP_SHOT_FROM {
            cop = true;
        }
        match (cop, car) {
            (true, _) if !matches!(self.shot, Shot::Cop(_)) => self.switch(false, facts),
            (false, true) if !matches!(self.shot, Shot::Car(_)) => self.switch(true, facts),
            (false, false) if self.shot != Shot::Gameplay => self.back_to_gameplay(),
            _ => {}
        }
        match &mut self.shot {
            Shot::Gameplay => {
                if self.lead_in > 0.0 {
                    self.slowdown.rate = self.lead_in;
                    self.slowdown.rising = true;
                }
                self.slowdown.advance(dt);
                self.speed = self.slowdown.sim_speed();
            }
            Shot::Car(shot) => {
                self.speed = track_car::SIM_SPEED;
                if shot.lost(&facts.car, self.car_clock, facts.scene) {
                    self.car_clock = -1.0;
                }
            }
            Shot::Cop(shot) => shot.advance(dt),
        }
    }

    /// Changes to a car shot (`car`) or the cop shot, placing its camera.
    fn switch(&mut self, car: bool, facts: &Facts<'_>) {
        self.shot = match car {
            true => {
                let framing = match (std::mem::take(&mut self.roadblock), facts.pursued) {
                    (true, _) => Framing::Roadblock,
                    (false, true) => Framing::Pursued,
                    (false, false) => Framing::Plain,
                };
                let shot = TrackCar::start(framing, &facts.car, self.last_eye, facts.camera.eye, facts.scene);
                self.last_eye = Some(shot.eye);
                Shot::Car(shot)
            }
            false => {
                let cop = nearest(facts.car.position, facts.cops);
                if cop.is_none() && self.cop_clock < rules::COP_SHOT_FROM {
                    self.cop_clock = -1.0;
                }
                Shot::Cop(TrackCop::start(facts.camera, cop, facts.scene))
            }
        };
        self.lead_in = 0.0;
        self.returning = false;
        self.cut = true;
    }

    fn back_to_gameplay(&mut self) {
        self.shot = Shot::Gameplay;
        self.slowdown = match self.returning {
            true => Slowdown::returning(),
            false => Slowdown::default(),
        };
        self.lead_in = 0.0;
        self.returning = false;
        self.cut = true;
    }

    /// Stops any shot at once (a scripted scene or the world's end takes the camera), with no slow motion after it.
    pub fn stop(&mut self) {
        let sounds = std::mem::take(&mut self.sounds);
        let last = self.last.take();
        let last_eye = self.last_eye;
        *self = Self { sounds, last, last_eye, cut: self.shot != Shot::Gameplay, ..Self::default() };
    }

    fn note(&mut self, what: &str) {
        log::info!("game moment camera: {what}");
        self.last = Some(what.to_owned());
    }

    /// Whether a shot (not the gameplay camera) plays.
    pub fn playing(&self) -> bool {
        self.shot != Shot::Gameplay
    }

    /// Whether the next step may cut to a shot that starts from the gameplay camera: a clock runs whose shot has not
    /// started yet (the pursuit shot's lead-in).
    pub fn wants_camera(&self) -> bool {
        let cop = self.cop_clock > 0.0 && !matches!(self.shot, Shot::Cop(_));
        let car = self.car_clock > 0.0 && !matches!(self.shot, Shot::Car(_));
        cop || car
    }

    /// A car shot plays: no new jump shot starts.
    pub fn car_shot(&self) -> bool {
        self.car_clock > 0.0
    }

    /// The HUD is hidden while the cop shot plays.
    pub fn hides_hud(&self) -> bool {
        matches!(self.shot, Shot::Cop(_))
    }

    /// The share of real time the world runs at.
    pub fn sim_speed(&self) -> f32 {
        self.speed
    }

    /// The camera of the shot, if one plays: eye, point looked at and lens (16-bit angle units).
    pub fn view(&self, car: &Car) -> Option<(Vec3, Vec3, f32)> {
        match &self.shot {
            Shot::Gameplay => None,
            Shot::Car(shot) => {
                let (target, fov) = shot.view(car);
                Some((shot.eye, target, fov))
            }
            Shot::Cop(shot) => Some(shot.view()),
        }
    }

    /// Whether the camera jumped since this was last called.
    pub fn take_cut(&mut self) -> bool {
        std::mem::take(&mut self.cut)
    }

    /// The moment sounds the shots posted since this was last called.
    pub fn take_sounds(&mut self) -> Vec<&'static str> {
        std::mem::take(&mut self.sounds)
    }

    /// One line for the console.
    pub fn status(&self) -> String {
        let now = match &self.shot {
            Shot::Gameplay => "gameplay camera".to_owned(),
            Shot::Car(_) => format!("car shot, {:.1} s left", self.car_clock),
            Shot::Cop(_) => format!("cop shot, {:.1} s left", self.cop_clock),
        };
        let last = self.last.as_deref().unwrap_or("none");
        format!("game moment camera: {now} at {:.0}% speed; last: {last}", self.speed * 100.0)
    }
}

/// The cop nearest the player within the cop shot's range.
fn nearest(player: Vec3, cops: &[Vec3]) -> Option<Vec3> {
    cops.iter()
        .copied()
        .filter(|cop| cop.distance(player) < track_cop::COP_RANGE)
        .min_by(|a, b| a.distance(player).total_cmp(&b.distance(player)))
}

#[cfg(test)]
#[path = "moment_cams/tests.rs"]
mod tests;

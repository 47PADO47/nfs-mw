//! What the car sound is told about the drive: the telemetry of the latest step, the collision events since
//! it last asked, and the scrape going on.

use blackbox_carsound::{CarInput, ScrapeKind};
use nfsmw_data::car::physics::SurfaceTable;
use nfsmw_data::sound::EventKind;

use super::input::DriveInput;
use super::sim::{CarSim, Telemetry};
use super::walls::Impact;
use crate::audio::{CarEvent, CarSoundState};

/// Wall impulse (N s) of a hit at full volume, and the least that sounds at all.
const IMPULSE_FULL: f32 = 30_000.0;
const IMPULSE_MIN: f32 = 800.0;
/// Physics steps between two wall hits that sound (a car pressed against a wall hits it every step).
const HIT_GAP: u32 = 12;
/// Speed (m/s) above which rubbing a wall scrapes, and at which it scrapes at full volume.
const SCRAPE_MIN: f32 = 3.0;
const SCRAPE_FULL: f32 = 30.0;
/// A light prop of this mass (kg) knocked over makes a hit of this size.
const PROP_FULL_MASS: f32 = 150.0;

#[derive(Debug, Default)]
pub struct SoundFeed {
    events: Vec<CarEvent>,
    scrape: Option<(ScrapeKind, f32)>,
    wheels: [blackbox_carsound::WheelInput; 4],
    up_dot: f32,
    surface: u32,
    pedals: (f32, f32),
    nos_pressed: bool,
    steps: u32,
    last_hit: u32,
}

impl SoundFeed {
    /// After a physics step: remember the wheels and the pedals, and turn what the step hit into events.
    pub fn after_step(
        &mut self,
        sim: &CarSim,
        surfaces: &SurfaceTable,
        impact: &Impact,
        input: &DriveInput,
        speed: f32,
    ) {
        self.steps += 1;
        self.wheels = sim.wheel_sounds(surfaces);
        self.up_dot = sim.up_dot();
        self.surface = sim.surface_tag().unwrap_or_else(|| SurfaceTable::hash_of("default"));
        self.pedals = (input.throttle, input.brake);
        self.nos_pressed = input.nos;

        let default = SurfaceTable::hash_of("default");
        if impact.impulse > IMPULSE_MIN && self.steps.saturating_sub(self.last_hit) >= HIT_GAP {
            self.last_hit = self.steps;
            if let Some((at, info)) = impact.deepest {
                // Where the car hit what, so a wall that should not be there can be found in the data.
                let p = crate::scenes::world::space::to_render(at.to_array());
                log::debug!(
                    "wall hit {:.0} N s at render ({:.1}, {:.1}, {:.1}): {info:?}",
                    impact.impulse,
                    p.x,
                    p.y,
                    p.z
                );
            }
            self.events.push(CarEvent {
                kind: EventKind::HitWorld,
                surface: default,
                magnitude: (impact.impulse / IMPULSE_FULL).clamp(0.0, 1.0),
                front: impact.front,
                smackable: false,
            });
        }
        for &(_, mass) in &impact.knocked {
            self.events.push(CarEvent {
                kind: EventKind::HitWorld,
                surface: default,
                magnitude: (mass / PROP_FULL_MASS).clamp(0.15, 0.6),
                front: true,
                smackable: true,
            });
        }
        let rubbing = impact.rigid > 0 && speed.abs() > SCRAPE_MIN;
        self.scrape = rubbing.then(|| (ScrapeKind::Wall, (speed.abs() / SCRAPE_FULL).clamp(0.0, 1.0)));
    }

    /// The state for the car sound. The events are handed over once.
    pub fn state(&mut self, car: &str, t: &Telemetry) -> CarSoundState {
        let span = t.red_line - t.idle;
        let input = CarInput {
            rpm_pct: if span > 0.0 { ((t.rpm - t.idle) / span).clamp(0.0, 1.0) } else { 0.0 },
            throttle: self.pedals.0,
            brake: self.pedals.1,
            // The transmission numbers reverse 0, neutral 1, first 2.
            gear: t.gear + 1,
            speed: t.speed_mps.abs(),
            // The loop follows the burn, not the button: a press that does nothing (too slow, no throttle, in
            // neutral, no nitrous) stays quiet.
            nos_active: t.nos_burning,
            nos_empty: self.nos_pressed && t.has_nos && t.nos <= 0.0,
            wheels: self.wheels,
            up_dot: self.up_dot,
            ..Default::default()
        };
        CarSoundState {
            car: car.to_owned(),
            input,
            events: std::mem::take(&mut self.events),
            scrape: self.scrape,
            surface: self.surface,
        }
    }
}

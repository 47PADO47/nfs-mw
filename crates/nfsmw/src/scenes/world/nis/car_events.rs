//! What a NIS scene's car events do to the car of their channel: the body pulses and the shake, the burnout, the brake
//! lock, the lights, the nitrous and the neutral revs, the release to free motion and back, and the repair. Spec:
//! `docs/specs/nis-events.md` section 2.2 (the original's handlers) and `race-reference-followup.md` (the pulses).
use glam::Quat;

/// A car event of a scene, for the car of the channel whose system fired it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CarEvent {
    Pitch {
        dip: f32,
        length: f32,
    },
    Roll {
        dip: f32,
        length: f32,
    },
    /// `ENISCarShake`: a roll vibration of the body of `dip` metres, one cycle every `rate` seconds, ramped in and out
    /// over `ramp` seconds, for `time` seconds.
    Shake {
        dip: f32,
        rate: f32,
        ramp: f32,
        time: f32,
    },
    Burnout(f32),
    /// `ENISBrakelock`: the front and the rear wheels locked (or freed).
    BrakeLock {
        front: bool,
        rear: bool,
    },
    /// `ENISLights`: the police lights and the head lights on or off.
    Lights {
        cop: bool,
        head: bool,
    },
    /// `ENISNitro`: the nitrous engaged or not.
    Nitro(bool),
    /// `ENISNeutralRev`: the engine revs in neutral at `throttle` (0 idle to 1 red line), reached at rate `speed`.
    NeutralRev {
        on: bool,
        throttle: f32,
        speed: f32,
    },
    /// `ENISDetach`: the car leaves its channel and moves on its own.
    Detach,
    /// `ENISReattach`: the car follows its channel again; `repair` also repairs it.
    Reattach {
        repair: bool,
    },
    /// `ENISCarDamageReset`: the car is repaired.
    DamageReset,
}

#[derive(Clone, Copy, Debug, Default)]
struct Pulse {
    dip: f32,
    start: f32,
    length: f32,
}

impl Pulse {
    fn angle(self, time: f32, span: f32) -> f32 {
        if self.length <= 0.0 {
            return 0.0;
        }
        let fraction = ((time - self.start) / self.length).clamp(0.0, 1.0);
        // The authored controller peaks at half its AI-angle displacement, then returns to rest.
        ai_angle(self.dip, span) * fraction.min(1.0 - fraction)
    }
}

/// A dip of `dip` metres over half of `span`, as the original's controller angle (radians over pi, used as radians).
fn ai_angle(dip: f32, span: f32) -> f32 {
    dip.atan2((span * 0.5).max(0.01)) / std::f32::consts::PI
}

#[derive(Clone, Copy, Debug, Default)]
struct Shake {
    dip: f32,
    rate: f32,
    ramp: f32,
    start: f32,
    length: f32,
}

impl Shake {
    /// The body roll of the shake at `time`: a cosine of one cycle per `rate` seconds, its whole cycles ramped up over
    /// the ramp's cycles (rounded up) and down over the last as many, zero outside `length`.
    fn angle(self, time: f32, width: f32) -> f32 {
        let elapsed = time - self.start;
        if self.length <= 0.0 || self.rate <= 0.0 || !(0.0..self.length).contains(&elapsed) {
            return 0.0;
        }
        let total = (self.length / self.rate).trunc();
        let cycle = (elapsed / self.rate).trunc();
        let ramp_cycles = (self.ramp / self.rate + 0.99).trunc();
        let mut envelope = 1.0;
        if ramp_cycles > 0.0 && cycle < ramp_cycles {
            envelope = cycle / ramp_cycles;
        }
        if ramp_cycles > 0.0 && cycle >= ramp_cycles && cycle > total - ramp_cycles {
            envelope = (total - cycle) / ramp_cycles;
        }
        let phase = (elapsed - self.rate * cycle) / self.rate;
        ai_angle(self.dip, width) * (phase * std::f32::consts::TAU).cos() * envelope
    }
}

/// What a channel's car events have set, at the scene time they fired.
#[derive(Clone, Copy, Debug)]
pub struct CarMotion {
    pitch: Pulse,
    roll: Pulse,
    shake: Shake,
    pub burnout: f32,
    /// The front and the rear wheels are locked.
    pub brake_lock: (bool, bool),
    /// The police lights and the head lights the scene asked for, `None` until it asks.
    pub lights: Option<(bool, bool)>,
    pub nitro: bool,
    /// The neutral revs: the throttle and the rate, while on.
    pub neutral_rev: Option<(f32, f32)>,
    /// The car follows its channel (`false` between a detach and a reattach).
    pub attached: bool,
    /// Counts the repairs the scene asked for, so the world repairs once per request.
    pub repairs: u32,
}

impl Default for CarMotion {
    fn default() -> Self {
        Self {
            pitch: Pulse::default(),
            roll: Pulse::default(),
            shake: Shake::default(),
            burnout: 0.0,
            brake_lock: (false, false),
            lights: None,
            nitro: false,
            neutral_rev: None,
            attached: true,
            repairs: 0,
        }
    }
}

impl CarMotion {
    pub fn apply(&mut self, event: CarEvent, time: f32) {
        match event {
            CarEvent::Pitch { dip, length } => self.pitch = Pulse { dip, start: time, length },
            CarEvent::Roll { dip, length } => self.roll = Pulse { dip, start: time, length },
            CarEvent::Shake { dip, rate, ramp, time: length } => {
                self.shake = Shake { dip, rate, ramp, start: time, length };
            }
            CarEvent::Burnout(speed) => self.burnout = speed,
            CarEvent::BrakeLock { front, rear } => self.brake_lock = (front, rear),
            CarEvent::Lights { cop, head } => self.lights = Some((cop, head)),
            CarEvent::Nitro(on) => self.nitro = on,
            CarEvent::NeutralRev { on, throttle, speed } => self.neutral_rev = on.then_some((throttle, speed)),
            CarEvent::Detach => self.attached = false,
            CarEvent::Reattach { repair } => {
                self.attached = true;
                self.repairs += u32::from(repair);
            }
            CarEvent::DamageReset => self.repairs += 1,
        }
    }

    /// The body's tilt at `time`: the roll pulse and the shake about the car's long axis, the pitch pulse about its
    /// cross axis.
    pub fn rotation(self, time: f32, wheelbase: f32, width: f32) -> Quat {
        let roll = self.roll.angle(time, width) + self.shake.angle(time, width);
        Quat::from_rotation_x(roll) * Quat::from_rotation_y(self.pitch.angle(time, wheelbase))
    }

    /// Whether the wheels of axle `rear` are locked: they do not turn.
    pub fn locked(self, rear: bool) -> bool {
        match rear {
            true => self.brake_lock.1,
            false => self.brake_lock.0,
        }
    }
}

pub fn slot_of(system: u32) -> Option<usize> {
    (0..16).find(|&slot| blackbox_attrib::vlt_hash(&slot_name(slot)) == system)
}

/// The channel name of car slot `slot`: `car1`..`car8`, then `cop1`..`cop8`.
pub fn slot_name(slot: usize) -> String {
    if slot < 8 { format!("car{}", slot + 1) } else { format!("cop{}", slot - 7) }
}

#[cfg(test)]
#[path = "car_events_tests.rs"]
mod tests;

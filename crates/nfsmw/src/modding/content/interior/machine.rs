//! The steps of entering and leaving a mod interior (docs/specs/mod-world-content.md, section 3.3). The machine decides and
//! returns [`Command`]s; the world carries them out (`scenes/world/entry.rs`). It holds no world, so the steps are tested
//! here.

use blackbox_modding::content::{Pose, Suspend, TimeOverride};

use crate::settings::TrafficSettings;
use crate::viewer::Environment;

/// The fade out before the load, and the fade in after the teleport, seconds. A design value of this port, not the
/// original's timing (the spec's manual test 19 measures it).
pub const FADE_SECS: f32 = 0.4;
/// How long the area may take to load before the entry is cancelled, seconds.
pub const LOAD_LIMIT_SECS: f32 = 10.0;

/// The clock as it was: the environment, and whether a mod had held it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clock {
    pub env: Environment,
    pub held: bool,
}

/// The world as it was before the entry: the car's place and heading (radians), the traffic settings and the clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
    pub pos: [f32; 3],
    pub heading: f32,
    pub traffic: TrafficSettings,
    pub clock: Clock,
}

/// What an entry needs from its interior: the residency index of its pack, the spawn, the clock and what stops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plan {
    pub tile: usize,
    pub spawn: Pose,
    pub time: Option<TimeOverride>,
    pub suspend: Suspend,
}

/// One step for the world to carry out, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    FadeOut,
    FadeIn,
    /// Keep the pack resident (`on`) or let it go back to the streaming rule (`!on`).
    Pin {
        tile: usize,
        on: bool,
    },
    /// Put the car at this place (render space, metres) and heading (radians), standing still.
    Teleport {
        pos: [f32; 3],
        heading: f32,
    },
    /// Stop what the interior names: traffic, cops and pursuit.
    Suspend(Suspend),
    SetClock(TimeOverride),
    /// Give back the traffic settings and the clock saved at entry.
    Restore(Snapshot),
    /// A line for the player, with the reason.
    Notice(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Stage {
    #[default]
    Outside,
    Fading {
        left: f32,
    },
    Loading {
        waited: f32,
    },
    Inside,
    Leaving {
        left: f32,
    },
}

/// The state of the one interior the player may be in.
#[derive(Debug, Default)]
pub struct Machine {
    stage: Stage,
    plan: Option<Plan>,
    saved: Option<Snapshot>,
}

impl Machine {
    pub fn is_outside(&self) -> bool {
        self.stage == Stage::Outside
    }

    pub fn is_inside(&self) -> bool {
        self.stage == Stage::Inside
    }

    /// The pack the entry waits for, while it waits.
    pub fn loading_tile(&self) -> Option<usize> {
        match (self.stage, self.plan) {
            (Stage::Loading { .. }, Some(plan)) => Some(plan.tile),
            _ => None,
        }
    }

    /// Start an entry from outside. Nothing happens when the player is already in or going in or out.
    pub fn begin(&mut self, plan: Plan, saved: Snapshot) -> Vec<Command> {
        if !self.is_outside() {
            return Vec::new();
        }
        self.stage = Stage::Fading { left: FADE_SECS };
        self.plan = Some(plan);
        self.saved = Some(saved);
        vec![Command::Pin { tile: plan.tile, on: true }, Command::FadeOut]
    }

    /// Start leaving from inside. Nothing happens from any other stage.
    pub fn leave(&mut self) -> Vec<Command> {
        if !self.is_inside() {
            return Vec::new();
        }
        self.stage = Stage::Leaving { left: FADE_SECS };
        vec![Command::FadeOut]
    }

    /// Advance by `dt` seconds. `ready` says whether the pack the entry waits for is resident.
    pub fn tick(&mut self, dt: f32, ready: bool) -> Vec<Command> {
        match self.stage {
            Stage::Fading { left } if left > dt => {
                self.stage = Stage::Fading { left: left - dt };
                Vec::new()
            }
            Stage::Fading { .. } => {
                self.stage = Stage::Loading { waited: 0.0 };
                self.load(0.0, ready)
            }
            Stage::Loading { waited } => self.load(waited + dt, ready),
            Stage::Leaving { left } if left > dt => {
                self.stage = Stage::Leaving { left: left - dt };
                Vec::new()
            }
            Stage::Leaving { .. } => self.finish(true, None),
            Stage::Outside | Stage::Inside => Vec::new(),
        }
    }

    fn load(&mut self, waited: f32, ready: bool) -> Vec<Command> {
        let Some(plan) = self.plan else { return Vec::new() };
        if ready {
            return self.enter(plan);
        }
        if waited < LOAD_LIMIT_SECS {
            self.stage = Stage::Loading { waited };
            return Vec::new();
        }
        self.finish(false, Some("the interior did not load; the entry is cancelled"))
    }

    fn enter(&mut self, plan: Plan) -> Vec<Command> {
        self.stage = Stage::Inside;
        let mut out = vec![
            Command::Teleport { pos: plan.spawn.pos, heading: plan.spawn.heading.to_radians() },
            Command::Suspend(plan.suspend),
        ];
        out.extend(plan.time.map(Command::SetClock));
        out.push(Command::FadeIn);
        out
    }

    /// Back to outside. `moved` says the car and the settings were changed, so they go back; `notice` is a line for the
    /// player.
    fn finish(&mut self, moved: bool, notice: Option<&'static str>) -> Vec<Command> {
        let saved = self.saved.take();
        let tile = self.plan.take().map(|plan| plan.tile);
        self.stage = Stage::Outside;
        let mut out: Vec<Command> = notice.map(Command::Notice).into_iter().collect();
        if let (true, Some(saved)) = (moved, saved) {
            out.push(Command::Teleport { pos: saved.pos, heading: saved.heading });
            out.push(Command::Restore(saved));
        }
        out.extend(tile.map(|tile| Command::Pin { tile, on: false }));
        out.push(Command::FadeIn);
        out
    }
}

/// The traffic settings inside an interior: no cars while traffic stops, no cop share while cops stop. The other
/// settings, and the share of cops while cops run, are kept.
pub fn traffic_while_inside(current: TrafficSettings, suspend: Suspend) -> TrafficSettings {
    TrafficSettings {
        cars: current.cars * u32::from(!suspend.traffic),
        cop_share: crate::settings::Percent(current.cop_share.0 * u8::from(!suspend.cops)),
        ..current
    }
}

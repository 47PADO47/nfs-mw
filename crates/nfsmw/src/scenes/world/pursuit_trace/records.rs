//! The JSON lines of a pursuit trace. The names follow the private original-game police recorder where an equivalent
//! exists (`samples.jsonl` / `events.jsonl`: `kind`, `time_ms`, `sim_time`, `player`, `heat`, `cops`, per cop `id`,
//! `generation`, `role`, `position`, `velocity`, `forward`, `speed`, `pursuit`, `desired_speed`, `drive_target`,
//! `goal`, `controls`); everything only the remake has is under `remake`. Vectors are physics space (x right, y up,
//! z forward), metres and m/s, as the original's.
use serde::Serialize;
use serde_json::Value;

use crate::scenes::world::ai::trace_frame::{CopFrame, PoliceFrame};

/// The capture-context flags the original recorder writes on every record: a remake trace is always live gameplay
/// (`flow_state` 6 is the original's in-game state), never a demo, the front end or a fixture.
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct Context {
    synthetic_probe: bool,
    observer_active: bool,
    flow_state: u32,
    ai_demo: u32,
    replay_demo: u32,
    joylog_replaying: u32,
    is_demo: bool,
    in_frontend: bool,
    capture_fixture: bool,
}

pub(super) const LIVE: Context = Context {
    synthetic_probe: false,
    observer_active: true,
    flow_state: 6,
    ai_demo: 0,
    replay_demo: 0,
    joylog_replaying: 0,
    is_demo: false,
    in_frontend: false,
    capture_fixture: false,
};

/// When a record was made: milliseconds since the session started (monotonic), the Unix time in milliseconds, and the
/// simulated seconds since the session started (it stops while the game is paused).
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct Stamp {
    pub time_ms: f64,
    pub wall_time_ms: u64,
    pub sim_time: f64,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct Player {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub forward: [f32; 3],
    pub speed: f32,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct Controls {
    steer: f32,
    gas: f32,
    brake: f32,
    nos: bool,
}

/// What only the remake knows of a cop.
#[derive(Clone, Debug, Serialize)]
pub(super) struct CopExtras {
    model: String,
    kind: &'static str,
    render_key: u32,
    searching: bool,
    fleeing: bool,
    engaged: bool,
    sees_player: bool,
    wrecked: bool,
    damage: f32,
    offscreen_s: f32,
    roadblock: Option<u32>,
    support: Option<u32>,
    /// A chase cop's own action this think and its road speed limit (m/s).
    action: Option<&'static str>,
    speed_limit: Option<f32>,
}

/// One cop, in the original's per-cop layout. `ai`, `pursuit_status`, `drivable_to_nav` and `nav` have no remake
/// equivalent and are null.
#[derive(Clone, Debug, Serialize)]
pub(super) struct Cop {
    id: String,
    generation: u32,
    vehicle: String,
    ai: Option<String>,
    role: &'static str,
    position: [f32; 3],
    velocity: [f32; 3],
    forward: [f32; 3],
    speed: f32,
    pursuit: Option<String>,
    pursuit_status: Option<u32>,
    desired_speed: f32,
    drive_target: [f32; 3],
    drivable_to_nav: Option<bool>,
    goal: Option<String>,
    nav: Option<Value>,
    controls: Controls,
    remake: CopExtras,
}

/// The cop id the original writes as `<pointer>:<generation>`. Remake entity ids are never reused, so the generation
/// is always 1.
pub(super) fn cop_id(key: u32) -> String {
    format!("remake-{key}:1")
}

/// Whether the cop counts as one of the chase's: the original marks a cop `pursuit_cop` while it has a pursuit.
/// Whether the cop counts as one of the chase's: not a patrol, and not a roadblock car (the original's roadblock cars
/// belong to their roadblock until the chase recruits one, which then joins as a chase cop).
pub(super) fn in_pursuit(cop: &CopFrame, frame: &PoliceFrame) -> bool {
    frame.pursuing && cop.kind != "patrol" && cop.kind != "roadblock"
}

impl Cop {
    pub fn role(&self) -> &'static str {
        self.role
    }
}

/// A cop in the original's layout; `member` says whether it counts as one of the chase's.
pub(super) fn cop(cop: &CopFrame, member: bool, pursuit: Option<&str>) -> Cop {
    Cop {
        id: cop_id(cop.key),
        generation: 1,
        vehicle: format!("remake-{}", cop.key),
        ai: None,
        role: match (member, cop.kind) {
            (true, _) => "pursuit_cop",
            (false, "patrol") => "patrol",
            (false, "roadblock") => "roadblock_cop",
            _ => "cop_unclassified",
        },
        position: cop.position,
        velocity: cop.velocity,
        forward: cop.forward,
        speed: cop.speed,
        pursuit: pursuit.filter(|_| member).map(str::to_owned),
        pursuit_status: None,
        desired_speed: cop.desired_speed,
        drive_target: cop.drive_target,
        drivable_to_nav: None,
        goal: cop.goal.clone(),
        nav: None,
        controls: Controls { steer: cop.steer, gas: cop.gas, brake: cop.brake, nos: cop.nos },
        remake: CopExtras {
            model: cop.model.clone(),
            kind: cop.kind,
            render_key: cop.render_key,
            searching: cop.searching,
            fleeing: cop.fleeing,
            engaged: cop.engaged,
            sees_player: cop.sees_player,
            wrecked: cop.wrecked,
            damage: cop.damage,
            offscreen_s: cop.offscreen_secs,
            roadblock: cop.roadblock,
            support: cop.support,
            action: cop.action,
            speed_limit: cop.speed_limit,
        },
    }
}

/// One 20 Hz sample.
#[derive(Clone, Debug, Serialize)]
pub(super) struct Sample {
    pub kind: &'static str,
    #[serde(flatten)]
    pub stamp: Stamp,
    pub player: Option<Player>,
    pub heat: Option<f32>,
    pub cops: Vec<Cop>,
    #[serde(flatten)]
    pub context: Context,
    pub remake: Value,
}

/// One lifecycle or pursuit event.
#[derive(Clone, Debug, Serialize)]
pub(super) struct Event {
    pub kind: &'static str,
    pub sequence: u64,
    #[serde(flatten)]
    pub stamp: Stamp,
    #[serde(flatten)]
    pub context: Context,
    pub id: Option<String>,
    pub generation: Option<u32>,
    pub vehicle: Option<String>,
    pub role: Option<&'static str>,
    pub pursuit: Option<String>,
    pub state: Option<Cop>,
    pub remake: Value,
}

/// The remake-only part of a sample: the chase's state, the counts, the roadblocks, the helicopter and the minimap.
pub(super) fn sample_extras(frame: &PoliceFrame, testing_heat: u8, paused: bool) -> Value {
    let strips: Vec<Value> = frame
        .strips
        .iter()
        .map(
            |s| serde_json::json!({"roadblock": s.roadblock, "index": s.index, "position": s.position, "axis": s.axis}),
        )
        .collect();
    let blips: Vec<Value> = frame
        .blips
        .iter()
        .map(|b| serde_json::json!({"style": b.style, "position_xz": b.position, "active": b.active}))
        .collect();
    let heli = frame.heli.as_ref().map(|h| {
        serde_json::json!({"position": h.position, "forward": h.forward, "velocity": h.velocity, "mode": h.mode,
            "in_sight": h.in_sight, "fuel_s": h.fuel_secs})
    });
    let roadblock_cars: Vec<Value> = frame
        .cops
        .iter()
        .filter(|c| c.roadblock.is_some())
        .map(|c| serde_json::json!({"roadblock": c.roadblock, "id": cop_id(c.key), "position": c.position, "forward": c.forward}))
        .collect();
    serde_json::json!({
        "pursuing": frame.pursuing,
        "heat_level": frame.heat_level,
        "testing_heat": testing_heat,
        "paused": paused,
        "phase": frame.phase,
        "cooldown": frame.cooldown,
        "cooldown_bar": frame.cooldown_bar,
        "secs_to_evade": frame.secs_to_evade,
        "in_sight": frame.in_sight,
        "contact": frame.contact,
        "busted_meter": frame.busted_meter,
        "end_reason": frame.end_reason,
        "backup_eta_s": frame.backup_eta,
        "backup_remaining": frame.backup_remaining,
        "support_order": frame.support_order,
        "lockout_s": frame.lockout_secs,
        "traffic_count": frame.traffic_cars,
        "racer_count": frame.racers,
        "cop_count": frame.cops.iter().filter(|c| !c.wrecked).count(),
        "cop_count_with_wrecks": frame.cops.len(),
        "roadblock_cars": roadblock_cars,
        "spike_strips": strips,
        "helicopter": heli,
        "minimap_blips": blips,
    })
}

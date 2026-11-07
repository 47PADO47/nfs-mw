//! A read-only snapshot of the police for the pursuit trace recorder (`scenes/world/pursuit_trace`). Positions,
//! velocities and directions are in physics space (x right, y up, z forward), as the original game's are.
use glam::Vec3;

use super::TrafficWorld;
use super::car::Role;
use crate::scenes::world::space;

/// One cop car.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::scenes::world) struct CopFrame {
    /// The entity id: stable for the car's life and never reused.
    pub key: u32,
    /// The id the speech events name the cop by.
    pub render_key: u32,
    pub model: String,
    /// patrol, chase, roadblock or support.
    pub kind: &'static str,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub forward: [f32; 3],
    pub speed: f32,
    pub desired_speed: f32,
    pub drive_target: [f32; 3],
    pub goal: Option<String>,
    pub steer: f32,
    pub gas: f32,
    pub brake: f32,
    pub nos: bool,
    pub searching: bool,
    pub fleeing: bool,
    pub engaged: bool,
    pub sees_player: bool,
    pub wrecked: bool,
    pub damage: f32,
    pub offscreen_secs: f32,
    pub roadblock: Option<u32>,
    pub support: Option<u32>,
    /// A chase cop's own action this think (`reverse`, `close`, `turnaround`, `road`) and its speed limit, m/s.
    pub action: Option<&'static str>,
    pub speed_limit: Option<f32>,
}

/// One spike strip of a standing roadblock.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::scenes::world) struct StripFrame {
    pub roadblock: u32,
    pub index: usize,
    pub position: [f32; 3],
    /// The strip frame's x axis (along the strip).
    pub axis: [f32; 3],
}

/// The helicopter.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::scenes::world) struct HeliFrame {
    pub position: [f32; 3],
    pub forward: [f32; 3],
    pub velocity: [f32; 3],
    pub mode: String,
    pub in_sight: bool,
    pub fuel_secs: f32,
}

/// One minimap blip of the traffic (cops, roadblocks, the helicopter): the style decides its look and colour.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::scenes::world) struct BlipFrame {
    pub style: &'static str,
    /// Physics x and z.
    pub position: [f32; 2],
    pub active: bool,
}

/// The police and the traffic around them at one moment.
#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::scenes::world) struct PoliceFrame {
    pub pursuing: bool,
    /// The chase's heat (1 to 10 with its fraction), or the car's heat outside a chase.
    pub heat: f32,
    pub heat_level: Option<u32>,
    pub phase: String,
    pub cooldown: String,
    pub cooldown_bar: f32,
    pub secs_to_evade: f32,
    pub in_sight: bool,
    pub contact: bool,
    pub busted_meter: f32,
    pub end_reason: String,
    pub backup_eta: Option<f32>,
    pub backup_remaining: Option<u32>,
    pub support_order: Option<String>,
    pub lockout_secs: f32,
    pub traffic_cars: usize,
    pub racers: usize,
    pub cops: Vec<CopFrame>,
    pub strips: Vec<StripFrame>,
    pub heli: Option<HeliFrame>,
    pub blips: Vec<BlipFrame>,
    /// The speech and radio inputs pushed since the last snapshot (`WorldEvent`, debug text), and how many the
    /// event ring dropped before they were read.
    pub speech: Vec<String>,
    pub speech_lost: u32,
}

fn physics(p: Vec3) -> [f32; 3] {
    space::to_physics(p)
}

impl TrafficWorld {
    /// The snapshot. `speech_seen` is the last speech event number read; it moves on to the newest.
    pub(in crate::scenes::world) fn trace_frame(&self, speech_seen: &mut u32) -> PoliceFrame {
        let pursuit = self.pursuit.as_ref();
        let mut blips = Vec::new();
        self.map_blips(&mut blips);
        let log = &self.speech_watch.log;
        let latest = log.latest();
        let speech: Vec<String> = log.since(*speech_seen).map(|event| format!("{event:?}")).collect();
        let speech_lost = (latest.saturating_sub(*speech_seen) as usize).saturating_sub(speech.len()) as u32;
        *speech_seen = latest;
        PoliceFrame {
            pursuing: pursuit.is_some(),
            heat: match pursuit {
                Some(_) => self.heat_value(),
                None => self.vehicle_heat(),
            },
            heat_level: pursuit.map(|p| p.heat),
            phase: format!("{:?}", self.pursuit_phase()),
            cooldown: format!("{:?}", self.cooldown_status()),
            cooldown_bar: self.cooldown_bar(),
            secs_to_evade: self.police.secs_to_evade(),
            in_sight: self.police.is_perp_in_sight(),
            contact: self.police.has_contact(),
            busted_meter: self.busted_meter(),
            end_reason: format!("{:?}", self.end_reason()),
            backup_eta: self.backup_eta(),
            backup_remaining: pursuit.map(|p| p.budget.remaining()),
            support_order: pursuit
                .and_then(|p| p.support.order())
                .map(|order| format!("{:?}", (order.strategy(), order.state(), order.denials()))),
            lockout_secs: self.cop_lockout_left(),
            traffic_cars: self.cars.iter().filter(|c| matches!(c.role, Role::Traffic) && !c.is_police()).count(),
            racers: self.cars.iter().filter(|c| c.is_racer()).count(),
            cops: self.cars.iter().filter(|c| c.is_police()).map(cop_frame).collect(),
            strips: self
                .spike_strips()
                .map(|(roadblock, index, frame)| StripFrame {
                    roadblock,
                    index,
                    position: physics(frame.w_axis.truncate()),
                    axis: physics(frame.x_axis.truncate().normalize_or_zero()),
                })
                .collect(),
            heli: self.helicopter_motion().map(|(position, forward, velocity)| {
                let status = self.helicopter_status();
                HeliFrame {
                    position: position.to_array(),
                    forward: forward.to_array(),
                    velocity: velocity.to_array(),
                    mode: status.map_or_else(|| "Unknown".into(), |s| format!("{:?}", s.mode)),
                    in_sight: status.is_some_and(|s| s.in_sight),
                    fuel_secs: status.map_or(0.0, |s| s.fuel_secs),
                }
            }),
            blips: blips
                .iter()
                .map(|blip| BlipFrame {
                    style: blip.style,
                    position: [-blip.position[1], blip.position[0]],
                    active: blip.active,
                })
                .collect(),
            speech,
            speech_lost,
        }
    }
}

fn cop_frame(car: &super::AiCar) -> CopFrame {
    let kind = match () {
        _ if car.is_roadblock() => "roadblock",
        _ if car.is_patrol() => "patrol",
        _ if car.support_serial().is_some() => "support",
        _ => "chase",
    };
    let controls = car.controls();
    let (target, desired) = car.drive_request();
    let goal = car.goal_drive().kind.map(|kind| format!("{kind:?}"));
    let drive = car.cop_drive_trace();
    CopFrame {
        key: car.key(),
        render_key: car.render_key(),
        model: car.name.clone(),
        kind,
        position: car.physics_position().to_array(),
        velocity: car.body().velocity.to_array(),
        forward: car.physics_forward().to_array(),
        speed: car.speed(),
        desired_speed: desired,
        drive_target: target.to_array(),
        goal,
        steer: controls.steer,
        gas: controls.gas,
        brake: controls.brake,
        nos: controls.nos,
        searching: car.is_searching(),
        fleeing: car.is_fleeing(),
        engaged: car.was_engaged(),
        sees_player: car.sees_player(),
        wrecked: car.is_wrecked(),
        damage: car.cop_damage(),
        offscreen_secs: car.offscreen_time(),
        roadblock: car.roadblock_group(),
        support: car.support_serial(),
        action: drive.map(|d| d.0),
        speed_limit: drive.map(|d| d.1),
    }
}

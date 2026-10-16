//! The cop manager: how many cops chase, which ones, and where they appear.
//! Spec: `docs/specs/ai-pursuit-cops.md` (§3, §4), `docs/specs/ai-pursuit-heat.md` (§2.7).

use std::rc::Rc;

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::EndReason;
use blackbox_pursuit::wave::{Slot, pick_needed};
use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use blackbox_vehicle::performance::Performance;
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;
use nfsmw_data::pursuit::{HeatRow, ground_cops};

use super::car::{AiCar, Role, Start};
use super::cop::{CopCar, CopState};
use super::traffic::Cruise;
use super::{Focus, TrafficModel, TrafficWorld};
use crate::scenes::world::drive::{CarRig, WorldGround};
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;
use crate::settings::bartender::BartenderSettings;

/// Spawn distance from the player, metres.
const MIN_DISTANCE: f32 = 150.0;
const MAX_DISTANCE: f32 = 400.0;
/// The angle off the player's heading the spawn point is rotated by: 60 degrees.
const SPREAD: f32 = std::f32::consts::FRAC_PI_3;
/// No cop starts nearer than this to another car.
const MIN_GAP: f32 = 40.0;
/// A cop this far from the player is taken out and spawned again closer.
const LOST_DISTANCE: f32 = 500.0;
/// A cop starts rolling at this speed, m/s.
pub(super) const START_SPEED: f32 = 15.0;
/// A cop car model with what limits how fast an AI drives it.
pub struct CopModel {
    pub model: TrafficModel,
    pub car: CopCar,
}

/// Where one cop car is put: the point on its road, what it faces, its rolling speed and the player's performance (the
/// cop's pursuit tuning).
#[derive(Debug, Clone, Copy)]
pub(super) struct CopSpot {
    pub at: Vec3,
    pub target: Vec3,
    pub speed: f32,
    pub player: Performance,
}

/// The chase: the heat rows of all levels, the heat it is at, and the spawn bookkeeping. The wave is the row of the
/// current heat, so a rise in heat brings the cop types of the higher row.
pub struct Pursuit {
    pub heat: u32,
    /// The rows of the heat levels, index = level - 1 (`nfsmw_data::pursuit::heat_rows`).
    rows: Vec<HeatRow>,
    player: Performance,
    spawn_timer: f32,
    involved: u32,
    /// The chase's support request and the cars it took (`support`).
    pub(super) support: super::support::Session,
}

impl Pursuit {
    pub fn new(heat: u32, rows: Vec<HeatRow>, player: Performance) -> Self {
        Self { heat, rows, player, spawn_timer: 0.0, involved: 0, support: Default::default() }
    }

    /// How the player's car performs, for the cops that chase it.
    pub(super) fn performance(&self) -> Performance {
        self.player
    }

    /// The row of the heat the chase is at now; `None` when there are no rows.
    pub fn row(&self) -> Option<&HeatRow> {
        let last = self.rows.len().checked_sub(1)?;
        self.rows.get((self.heat.max(1) as usize - 1).min(last))
    }

    /// Follows the police heat level: a new level takes effect at once. Returns true when the level changed.
    pub fn follow_heat(&mut self, level: u32) -> bool {
        if level == self.heat {
            return false;
        }
        self.heat = level;
        true
    }

    /// The car types of the wave at the current heat (without the helicopter) with how many of each.
    pub fn wave(&self) -> impl Iterator<Item = (&str, u32, u32)> {
        self.row().into_iter().flat_map(ground_cops).map(|c| (c.name.as_str(), c.count, c.chance))
    }

    /// How many cops the chase wants now: the whole wave, but no more than the cops still to evade (the level's cap
    /// from the Bartender file, or the install's value without one), and no more than the chase's cop limit.
    pub fn wanted(&self, settings: &BartenderSettings) -> usize {
        let Some(row) = self.row() else { return 0 };
        let nominal: u32 = self.wave().map(|w| w.1).sum();
        let cap = settings.cop_cap(self.heat).unwrap_or(row.full_engagement_cops).max(1);
        (nominal.min(cap) as usize).min(settings.max_cops as usize)
    }
}

impl TrafficWorld {
    /// Starts a chase at `heat` with the wave rows `rows` (one per level); `player` is how the player's car performs.
    pub fn start_pursuit(&mut self, heat: u32, rows: Vec<HeatRow>, player: Performance) {
        // The cop waves the mods set for the session take the install's rows here, so the next chase uses them.
        let rows = crate::modding::cop_rows::with_heat_overrides(rows);
        self.pursuit = Some(Pursuit::new(heat, rows, player));
        self.police.begin(heat);
    }

    /// Ends the chase without a result (the `pursuit off` command) and removes the cops.
    pub fn stop_pursuit(&mut self) {
        self.end_pursuit(EndReason::None);
    }

    /// Ends the chase with `reason` and removes the cops. The reason stays readable after it (the end card, the camera).
    pub fn end_pursuit(&mut self, reason: EndReason) {
        self.pursuit = None;
        self.police.end(reason);
        self.cars.retain(|c| !c.is_cop());
        self.clear_spikes();
    }

    pub fn add_cop_model(&mut self, model: CopModel) {
        self.known.insert(model.model.name.to_ascii_lowercase());
        self.cop_models.push(model);
    }

    pub fn has_cop_model(&self, name: &str) -> bool {
        self.cop_models.iter().any(|m| m.model.name.eq_ignore_ascii_case(name))
    }

    pub fn has_helicopter_rig(&self) -> bool {
        self.helicopter_rig.is_some()
    }

    pub fn set_helicopter_rig(&mut self, rig: Rc<CarRig>) {
        self.helicopter_rig = Some(rig);
    }

    /// Keeps the wave on the road: removes cops that fell far behind and sends in the next one when its
    /// time has come. `sim_dt` is the simulated time since the last call.
    pub(super) fn update_pursuit(
        &mut self,
        sim_dt: f32,
        focus: Focus,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) {
        let Some(player) = focus.body else { return };
        self.cars.retain(|car| {
            let p = car.position();
            !car.is_cop() || (p.x - focus.position[0]).hypot(p.y - focus.position[1]) < LOST_DISTANCE
        });
        self.update_support(sim_dt, focus, collision, surfaces);
        let level = self.police.level();
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        if let Some(level) = level
            && pursuit.follow_heat(level)
        {
            let wave: Vec<&str> = pursuit.wave().map(|w| w.0).collect();
            log::info!("pursuit: heat {level}, the wave is now {}", wave.join(", "));
        }
        pursuit.spawn_timer -= sim_dt;
        let alive = self.cars.iter().filter(|c| c.is_cop()).count();
        if alive >= pursuit.wanted(self.police.settings()) || pursuit.spawn_timer > 0.0 {
            return;
        }
        // The next type of the current wave, weighted by its chance among those the wave still needs.
        let pick = {
            let slots: Vec<Slot> = pursuit.wave().map(|(name, count, chance)| Slot { name, count, chance }).collect();
            let have =
                |name: &str| self.cars.iter().filter(|c| c.is_cop() && c.name.eq_ignore_ascii_case(name)).count();
            let roll = self.rng.next_f32();
            pick_needed(&slots, |name| have(name) as u32, roll).map(|i| slots[i].name.to_owned())
        };
        let Some(name) = pick else { return };
        let Some((first_four, later)) =
            pursuit.row().map(|r| (r.time_between_first_four_spawns, r.time_between_cop_spawns))
        else {
            return;
        };
        let involved = pursuit.involved;
        let delay = match involved < 3 {
            true => first_four,
            false => later,
        };
        let performance = pursuit.player;
        if self.spawn_cop(&name, focus, player.position, performance, collision, surfaces)
            && let Some(p) = self.pursuit.as_mut()
        {
            p.involved += 1;
            p.spawn_timer = delay;
        }
    }

    /// Puts a cop car of `name` on the cop roads 150 to 400 m from the player, facing the player.
    fn spawn_cop(
        &mut self,
        name: &str,
        focus: Focus,
        target: Vec3,
        player: Performance,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let rng = &mut self.rng;
        let distance = MIN_DISTANCE + rng.next_f32() * (MAX_DISTANCE - MIN_DISTANCE);
        let angle = focus.heading + (rng.next_f32() * 2.0 - 1.0) * SPREAD;
        let at = Vec3::new(focus.position[0] + distance * angle.cos(), focus.position[1] + distance * angle.sin(), 0.0);
        let physics_at = Vec3::from(space::to_physics(at));
        let spot = CopSpot { at: physics_at, target, speed: START_SPEED, player };
        self.spawn_cop_at(name, spot, collision, surfaces)
    }

    /// Puts a cop car of `name` on the cop road at `spot.at`, rolling at `spot.speed` and facing `spot.target`. The spawn path
    /// of every cop, the wave's and the support cars' (`ai/support/spawn.rs`). Returns whether the car stands.
    pub(super) fn spawn_cop_at(
        &mut self,
        name: &str,
        spot: CopSpot,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let CopSpot { at: physics_at, target, speed, player } = spot;
        let Some(model) = self.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case(name)) else {
            return false;
        };
        let toward = Vec3::new(target.x - physics_at.x, 0.0, target.z - physics_at.z);
        let filter = SegmentFilter { cop: true, no_decision: true, ..SegmentFilter::default() };
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, filter);
        if !nav.init_at_point(&self.network, &self.index, physics_at, toward, false) {
            return false;
        }
        if self.cars.iter().any(|c| c.physics_position().distance(nav.position) < MIN_GAP) {
            return false;
        }
        let render = space::to_render(nav.position.to_array());
        let facing = space::to_render(nav.forward.to_array());
        let spawn = Spawn { position: render, heading: facing.y.atan2(facing.x) };
        nav.half_width = model.model.physics.spec.dimension.x;
        nav.enable_trail(&self.network);
        let state = CopState::new(model.car, player, speed);
        let start =
            Start { stagger: (self.rng.next_f32() * 8.0) as u32, speed, cruise: Cruise::default(), patrol: false };
        let Some(car) = AiCar::place(
            &model.model,
            Role::Cop(Box::new(state)),
            nav,
            spawn,
            start,
            &WorldGround { collision, surfaces },
        ) else {
            return false;
        };
        log::info!(
            "pursuit: {} at ({:.0}, {:.0}), {:.0} m from the player",
            car.name,
            render.x,
            render.y,
            toward.length()
        );
        self.cars.push(car);
        true
    }

    /// One line for the `pursuit status` command.
    pub fn pursuit_status(&self) -> String {
        let Some(p) = &self.pursuit else { return "no pursuit".into() };
        let cops: Vec<String> = self.cars.iter().filter(|c| c.is_cop()).map(|c| c.name.clone()).collect();
        let infractions: Vec<String> = self.police.infractions().iter().map(|i| format!("{i:?}")).collect();
        let formation = self.police.last_formation().map_or("none yet".to_owned(), |f| format!("{f:?}"));
        let to_evade = self.pursuit_feed().map_or(0.0, |feed| feed.secs_to_evade);
        format!(
            "heat {}: {} of {} cops ({}); formation {formation}; evade in {to_evade:.0} s; infractions {} for ${}{}",
            p.heat,
            cops.len(),
            p.wanted(self.police.settings()),
            cops.join(", "),
            infractions.len(),
            self.police.infractions().total_fines(),
            if infractions.is_empty() { String::new() } else { format!(" ({})", infractions.join(", ")) },
        )
    }
}

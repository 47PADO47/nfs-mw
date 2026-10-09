//! The cop manager: how many cops chase, which ones, and where they appear.
//! Spec: `docs/specs/ai-pursuit-cops.md` (§3, §4), `docs/specs/ai-pursuit-heat.md` (§2.7).

use blackbox_collision::CollisionWorld;
use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use blackbox_vehicle::performance::Performance;
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;
use nfsmw_data::pursuit::HeatRow;

use super::car::{AiCar, Role, Start};
use super::cop::{CopCar, CopState};
use super::{Focus, TrafficModel, TrafficWorld};
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// Cop cars chasing at once, at most.
const MAX_COPS: usize = 8;
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
const START_SPEED: f32 = 15.0;
/// The helicopter record of a wave is not a car.
const HELICOPTER: &str = "copheli";

/// A cop car model with what limits how fast an AI drives it.
pub struct CopModel {
    pub model: TrafficModel,
    pub car: CopCar,
}

/// The chase: the heat row of the wave and the spawn bookkeeping.
pub struct Pursuit {
    pub heat: u32,
    row: HeatRow,
    player: Performance,
    spawn_timer: f32,
    involved: u32,
}

impl Pursuit {
    pub fn new(heat: u32, row: HeatRow, player: Performance) -> Self {
        Self { heat, row, player, spawn_timer: 0.0, involved: 0 }
    }

    /// The car types of the wave (without the helicopter) with how many of each.
    pub fn wave(&self) -> impl Iterator<Item = (&str, u32, u32)> {
        self.row.cops.iter().filter(|c| c.name != HELICOPTER).map(|c| (c.name.as_str(), c.count, c.chance))
    }

    /// How many cops the chase wants now: the whole wave, but no more than the cops still to evade.
    pub fn wanted(&self) -> usize {
        let nominal: u32 = self.wave().map(|w| w.1).sum();
        (nominal.min(self.row.full_engagement_cops.max(1)) as usize).min(MAX_COPS)
    }
}

impl TrafficWorld {
    /// Starts a chase at `heat` with the wave of `row`; `player` is how the player's car performs.
    pub fn start_pursuit(&mut self, heat: u32, row: HeatRow, player: Performance) {
        self.pursuit = Some(Pursuit::new(heat, row, player));
    }

    /// Ends the chase and removes the cops.
    pub fn stop_pursuit(&mut self) {
        self.pursuit = None;
        self.cars.retain(|c| !c.is_cop());
    }

    pub fn add_cop_model(&mut self, model: CopModel) {
        self.cop_models.push(model);
    }

    pub fn has_cop_model(&self, name: &str) -> bool {
        self.cop_models.iter().any(|m| m.model.name.eq_ignore_ascii_case(name))
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
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        pursuit.spawn_timer -= sim_dt;
        let alive = self.cars.iter().filter(|c| c.is_cop()).count();
        if alive >= pursuit.wanted() || pursuit.spawn_timer > 0.0 {
            return;
        }
        // The next type, weighted by its chance among those the wave still needs.
        let mut weights: Vec<(String, u32)> = Vec::new();
        for (name, count, chance) in pursuit.wave() {
            let have = self.cars.iter().filter(|c| c.is_cop() && c.name.eq_ignore_ascii_case(name)).count() as u32;
            if have < count {
                weights.push((name.to_owned(), chance.max(1)));
            }
        }
        let total: u32 = weights.iter().map(|w| w.1).sum();
        if total == 0 {
            return;
        }
        let mut roll = (self.rng.next_f32() * total as f32) as u32;
        let name = weights
            .iter()
            .find(|(_, w)| {
                let hit = roll < *w;
                roll = roll.saturating_sub(*w);
                hit
            })
            .map(|w| w.0.clone())
            .unwrap_or_else(|| weights[0].0.clone());
        let involved = pursuit.involved;
        let delay = match involved < 3 {
            true => pursuit.row.time_between_first_four_spawns,
            false => pursuit.row.time_between_cop_spawns,
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
        let Some(model) = self.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case(name)) else {
            return false;
        };
        let rng = &mut self.rng;
        let distance = MIN_DISTANCE + rng.next_f32() * (MAX_DISTANCE - MIN_DISTANCE);
        let angle = focus.heading + (rng.next_f32() * 2.0 - 1.0) * SPREAD;
        let at = Vec3::new(focus.position[0] + distance * angle.cos(), focus.position[1] + distance * angle.sin(), 0.0);
        let physics_at = Vec3::from(space::to_physics(at));
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
        let state = CopState::new(model.car, player, START_SPEED);
        let start = Start { stagger: (rng.next_f32() * 8.0) as u32, speed: START_SPEED };
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
        log::info!("pursuit: {} at ({:.0}, {:.0}), {:.0} m from the player", car.name, render.x, render.y, distance);
        self.cars.push(car);
        true
    }

    /// One line for the `pursuit status` command.
    pub fn pursuit_status(&self) -> String {
        let Some(p) = &self.pursuit else { return "no pursuit".into() };
        let cops: Vec<String> = self.cars.iter().filter(|c| c.is_cop()).map(|c| c.name.clone()).collect();
        format!("heat {}: {} of {} cops ({})", p.heat, cops.len(), p.wanted(), cops.join(", "))
    }
}

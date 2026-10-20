//! The point each cop drives at this step: the player, a collapse ring slot, a formation slot, or a block slot or the rear
//! quarter for the tactics the chase allows. A cop on a goal (the finisher's pit, box-in or rolling block, or the collapse's
//! pull-over) drives by that goal (`goal_aims.rs`), which also sets its speed and arrival. The rules are pure
//! (`blackbox_pursuit::tactics::plan`, `formation::slots::{assign, assign_slots}`, `formation::collapse::ring_slots`); this
//! file reads the cars, the heat, the formation and the player's speed. Spec: `docs/specs/ai-pursuit-tactics.md`,
//! `docs/specs/police-tactics.md`, `docs/specs/ai-pursuit-formations.md`, `docs/specs/ai-pursuit-goals.md`.

use blackbox_pursuit::formation::collapse::{self, ring_slots};
use blackbox_pursuit::formation::slots::{assign, assign_slots};
use blackbox_pursuit::goals::Tuning;
use blackbox_pursuit::goals::approach::{CopState, Step};
use blackbox_pursuit::tactics::plan;
use blackbox_roads::Body;
use glam::{Vec2, Vec3};

use super::car::Role;
use super::goal_aims::{self, Goal};
use crate::scenes::world::drive::STEP;

/// From this heat level cops block the road (the heat tables' roadblock rows; the level is unconfirmed).
const BLOCK_FROM_LEVEL: u32 = 3;
/// From this heat level cops pit the player (unconfirmed).
const PIT_FROM_LEVEL: u32 = 4;
/// Cops this far from the player (metres) can join a collapse ring (`ai-pursuit-formations.md` §5.1).
const COLLAPSE_RANGE: f32 = 60.0;

/// What a car is asked to drive this step.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Plan {
    /// The chase's own aim: a formation slot, a tactic's point, or the player (`None`).
    Chase(Option<Vec2>),
    /// A goal, which sets the cop's speed and arrival too.
    Goal(Goal),
}

impl super::TrafficWorld {
    /// The aim of every car this step, by car index, into `out` (cleared first); `None` is the player. A cop on a goal drives
    /// by the goal's step, which is kept in the car (`car/goal_drive.rs`) for its think and its brake. The aims are in the map
    /// plane lifted to the player's height.
    pub(super) fn cop_aims(&self, player: &Body, out: &mut Vec<Option<Vec3>>) {
        if !self.police.is_perp_in_sight() {
            out.clear();
            out.resize(self.cars.len(), None);
            for car in &self.cars {
                car.set_goal_drive(None, None);
            }
            return;
        }
        let tuning = goal_aims::tuning_for(self.police.level().unwrap_or(1));
        let plans = self.cop_plans(player);
        out.clear();
        out.resize(self.cars.len(), None);
        for (me, order) in plans.into_iter().enumerate() {
            let kind = match order {
                Plan::Goal(goal) => Some(goal.kind()),
                _ => None,
            };
            let (aim, step) = match order {
                Plan::Chase(aim) => (aim, None),
                Plan::Goal(goal) => match self.goal_step(me, goal, player, &tuning) {
                    Some((aim, step)) => (Some(aim), Some(step)),
                    None => (None, None),
                },
            };
            self.cars[me].set_goal_drive(step, kind);
            out[me] = aim.map(|a| Vec3::new(a.x, player.position.y, a.y));
        }
    }

    /// What each car is asked to drive this step, by car index. Only cops in a chase have one.
    fn cop_plans(&self, player: &Body) -> Vec<Plan> {
        let mut plans = vec![Plan::Chase(None); self.cars.len()];
        let Some(level) = self.police.level() else { return plans };
        // Support cars take no formation slot, the finisher or the collapse: they keep the plan they start with (the chase's
        // aim, or their own goal in `car/support_drive.rs`). Spec `ai-pursuit-formations.md` section 3.
        let cops: Vec<usize> = (0..self.cars.len())
            .filter(|&i| self.cars[i].is_cop() && self.cars[i].support_serial().is_none())
            .collect();
        let positions: Vec<Vec2> = cops
            .iter()
            .map(|&i| {
                let p = self.cars[i].avoidable().position;
                Vec2::new(p.x, p.z)
            })
            .collect();
        let player_xz = Vec2::new(player.position.x, player.position.z);
        let forward = player.forward.normalize_or_zero();
        let right = Vec2::new(forward.y, -forward.x);
        let speed_kmh = Vec2::new(player.velocity.x, player.velocity.z).length() * 3.6;
        let collapse = collapse::level(level);
        let cop_plans = if self.police.finisher_running() {
            self.finisher_plans(&positions, player_xz, &goal_aims::tuning_for(level))
        } else if self.police.last_formation().is_some() && speed_kmh < collapse.speed_kmh {
            collapse_plans(&positions, player_xz, forward, right, collapse)
        } else {
            self.chase_plans(&positions, player_xz, forward, level)
        };
        for (&car, order) in cops.iter().zip(cop_plans) {
            plans[car] = order;
        }
        plans
    }

    /// The finisher: the cops nearest the target take the formation's slots, and each takes its goal (pit, box-in or rolling
    /// block) at that slot. The others drive at the player, as does every cop of a formation without a finisher.
    fn finisher_plans(&self, positions: &[Vec2], player: Vec2, tuning: &Tuning) -> Vec<Plan> {
        let none = vec![Plan::Chase(None); positions.len()];
        let Some(formation) = self.police.last_formation() else { return none };
        let Some(kind) = goal_aims::goal_for_formation(formation) else { return none };
        assign_slots(formation, positions, player)
            .into_iter()
            .map(|slot| match slot.and_then(|s| goal_aims::finisher_goal(kind, s, tuning)) {
                Some(goal) => Plan::Goal(goal),
                None => Plan::Chase(None),
            })
            .collect()
    }

    /// The chase's own aims: a formation's pursuit slots, or the tactics of the heat when no formation is on.
    fn chase_plans(&self, positions: &[Vec2], player: Vec2, forward: Vec2, level: u32) -> Vec<Plan> {
        let aims = match self.police.last_formation() {
            Some(formation) => assign(formation, positions, player, forward),
            None => plan(positions, player, forward, level >= BLOCK_FROM_LEVEL, level >= PIT_FROM_LEVEL),
        };
        aims.into_iter().map(Plan::Chase).collect()
    }

    /// One step of cop `me`'s goal: the aim it steers at and the goal's step, from the cop's own state and the player's.
    /// `None` for a car that is not a cop.
    fn goal_step(&self, me: usize, goal: Goal, player: &Body, tuning: &Tuning) -> Option<(Vec2, Step)> {
        let car = &self.cars[me];
        let Role::Cop(cop) = &car.role else { return None };
        let position = car.avoidable().position;
        let forward_speed = car.speed();
        let state = CopState {
            position: Vec2::new(position.x, position.z),
            forward_speed,
            limit: car.goal_drive().limit,
            dt: STEP,
        };
        let target = goal_aims::target_state(player, &cop.car.performance, forward_speed);
        let vehicle = goal_aims::vehicle(cop.car.ai);
        log::trace!("cop {me} goal {:?} at {:?}", goal.kind(), state.position);
        Some(goal_aims::step_goal(goal, &state, &target, vehicle, tuning))
    }
}

/// The collapse: the cops within range take the ring slots around the player (the pull-over goal); the others keep the chase.
fn collapse_plans(positions: &[Vec2], player: Vec2, forward: Vec2, right: Vec2, level: collapse::Level) -> Vec<Plan> {
    let mut plans = vec![Plan::Chase(None); positions.len()];
    let near: Vec<usize> =
        (0..positions.len()).filter(|&k| (positions[k] - player).length() <= COLLAPSE_RANGE).collect();
    let offsets: Vec<Vec2> = near
        .iter()
        .map(|&k| {
            let rel = positions[k] - player;
            Vec2::new(rel.dot(right), rel.dot(forward))
        })
        .collect();
    for (slot, offset) in ring_slots(&offsets, level) {
        plans[near[slot]] = Plan::Goal(Goal::Ring(player + right * offset.x + forward * offset.y));
    }
    plans
}

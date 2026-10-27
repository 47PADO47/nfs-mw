//! The point each cop drives at this step: the player, a collapse ring slot, a formation slot, or an extra-car grid slot.
//! A cop on a goal (the finisher's pit, box-in or rolling block, or the collapse's
//! pull-over) drives by that goal (`goal_aims.rs`), which also sets its speed and arrival. The rules are pure
//! (`blackbox_pursuit::formation::assignment`, `formation::collapse::ring_slots`); this
//! file reads the cars, the heat, the formation and the player's speed. Spec: `docs/specs/ai-pursuit-tactics.md`,
//! `docs/specs/police-tactics.md`, `docs/specs/ai-pursuit-formations.md`, `docs/specs/ai-pursuit-goals.md`.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::formation::collapse::{self, ring_slots};
use blackbox_pursuit::goals::Tuning;
use blackbox_pursuit::goals::approach::{CopState, Step};
use blackbox_roads::Body;
use glam::{Vec2, Vec3};

use super::car::Role;
use super::goal_aims::{self, Goal};

mod membership;
pub(super) use membership::Membership;
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
    pub(super) fn cop_aims(&self, player: &Body, collision: &CollisionWorld, out: &mut Vec<Option<Vec3>>) {
        if !self.police.is_perp_in_sight() || self.police.is_busted() {
            self.formation_membership.borrow_mut().collapsing = false;
            out.clear();
            out.resize(self.cars.len(), None);
            for car in &self.cars {
                car.set_goal_drive(None, None);
            }
            return;
        }
        let tuning = self.police.goal_tuning();
        let cast = crate::scenes::world::drive::sight::sight_cast(collision);
        let plans = self.cop_plans(player, &|from, to| cast(from, to).is_none());
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
    fn cop_plans(&self, player: &Body, clear: &dyn Fn(Vec3, Vec3) -> bool) -> Vec<Plan> {
        let mut plans = vec![Plan::Chase(None); self.cars.len()];
        let Some(_) = self.police.level() else { return plans };
        // Support cars take no formation slot, the finisher or the collapse: they keep the plan they start with (the chase's
        // aim, or their own goal in `car/support_drive.rs`). Spec `ai-pursuit-formations.md` section 3.
        let player_xz = Vec2::new(player.position.x, player.position.z);
        let forward = blackbox_pursuit::goals::approach::motion_vector(
            Vec2::new(player.velocity.x, player.velocity.z),
            player.forward,
        )
        .normalize_or_zero();
        let layout = self.formation_layout(player.position, forward, clear);
        let right = Vec2::new(forward.y, -forward.x);
        let speed_kmh = Vec2::new(player.velocity.x, player.velocity.z).length() * 3.6;
        let mut collapse = self.police.collapse_tuning();
        if self.police.is_jerk() {
            collapse.speed_kmh = 125.0;
        }
        let collapsing = self.police.last_formation().is_some()
            && layout.slots.iter().any(Option::is_some)
            && speed_kmh < collapse.speed_kmh
            && layout
                .positions
                .iter()
                .zip(&layout.eligible)
                .any(|(p, &eligible)| eligible && p.distance(player_xz) < COLLAPSE_RANGE);
        self.formation_membership.borrow_mut().collapsing = !self.police.finisher_running() && collapsing;
        let cop_plans = match (self.police.finisher_running(), collapsing) {
            (true, _) => self.finisher_plans(&layout.slots, &layout.aims, &self.police.goal_tuning()),
            (false, true) => collapse_plans(&layout.positions, &layout.eligible, player_xz, forward, right, collapse),
            _ => layout.aims.iter().map(|&aim| Plan::Chase(Some(aim))).collect(),
        };
        for (&car, order) in layout.cops.iter().zip(cop_plans) {
            plans[car] = order;
        }
        plans
    }

    /// The finisher: retain each formation member's slot and goal. Extra cars keep their grid assignment.
    fn finisher_plans(&self, slots: &[Option<usize>], aims: &[Vec2], tuning: &Tuning) -> Vec<Plan> {
        let none = aims.iter().map(|&aim| Plan::Chase(Some(aim))).collect();
        let Some(formation) = self.police.last_formation() else { return none };
        let Some(kind) = goal_aims::goal_for_formation(formation) else { return none };
        slots
            .iter()
            .zip(aims)
            .map(|(slot, &aim)| match slot.and_then(|s| goal_aims::finisher_goal(kind, s, tuning)) {
                Some(goal) => Plan::Goal(goal),
                None => Plan::Chase(Some(aim)),
            })
            .collect()
    }

    /// One step of cop `me`'s goal: the aim it steers at and the goal's step, from the cop's own state and the player's.
    /// `None` for a car that is not a cop.
    fn goal_step(&self, me: usize, goal: Goal, player: &Body, tuning: &Tuning) -> Option<(Vec2, Step)> {
        let car = &self.cars[me];
        let Role::Cop(cop) = &car.role else { return None };
        let position = car.avoidable().position;
        let forward_speed = car.speed();
        let previous = car.goal_drive();
        if previous.kind == Some(goal.kind())
            && !car.cop_think_due()
            && let Some(command) = previous.command()
        {
            return Some((
                command.target_point.unwrap_or(Vec2::new(position.x, position.z)),
                Step { command, limit: previous.limit },
            ));
        }
        let state = CopState {
            position: Vec2::new(position.x, position.z),
            forward_speed,
            limit: match previous.kind == Some(goal.kind()) {
                true => previous.limit,
                false => forward_speed,
            },
            dt: car.next_think_delta(self.step_delta),
        };
        let performance = self.pursuit.as_ref()?.performance();
        let mut target = goal_aims::target_state(player, &performance, forward_speed);
        target.jerk = self.police.is_jerk();
        let vehicle = goal_aims::vehicle(cop.car.ai);
        let aggression = blackbox_pursuit::goals::pull_over::aggression(tuning.collapse_aggression, target.jerk);
        let (absolute, relative) = blackbox_pursuit::goals::pull_over::separation_scales(aggression);
        let others = self
            .cars
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != me)
            .map(|(_, other)| other.avoidable())
            .chain(std::iter::once(*player));
        let separation = super::cop::separation::steer(&car.body(), others, absolute, relative);
        log::trace!("cop {me} goal {:?} at {:?}", goal.kind(), state.position);
        Some(goal_aims::step_goal(goal, &state, &target, vehicle, tuning, separation))
    }
}

/// The collapse: the cops within range take the ring slots around the player (the pull-over goal); the others keep the chase.
fn collapse_plans(
    positions: &[Vec2],
    eligible: &[bool],
    player: Vec2,
    forward: Vec2,
    right: Vec2,
    level: collapse::Level,
) -> Vec<Plan> {
    let mut plans = vec![Plan::Chase(None); positions.len()];
    let near: Vec<usize> =
        (0..positions.len()).filter(|&k| eligible[k] && (positions[k] - player).length() < COLLAPSE_RANGE).collect();
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

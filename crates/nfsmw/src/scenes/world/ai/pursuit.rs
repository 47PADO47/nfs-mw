//! The cop manager: how many cops chase, which ones, and where they appear.
//! Spec: `docs/specs/ai-pursuit-cops.md` (§3, §4), `docs/specs/ai-pursuit-heat.md` (§2.7).

use std::rc::Rc;

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::EndReason;
use blackbox_pursuit::backup::Budget;
use blackbox_pursuit::wave::{Slot, adjusted_counts, pick_needed, want_cops};
use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use blackbox_vehicle::performance::Performance;
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;
use nfsmw_data::pursuit::{BackupRow, HeatRow, ground_cops};

use super::car::{AiCar, Role, Start};
use super::cop::{CopCar, CopState};
use super::traffic::Cruise;
use super::{Focus, TrafficModel, TrafficWorld};
use crate::modding::cop_rows::without_models;
use crate::scenes::world::drive::{CarRig, WorldGround};
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;
use crate::settings::bartender::BartenderSettings;

/// Spawn distance from the player, metres.
const MIN_DISTANCE: f32 = 150.0;
const MAX_DISTANCE: f32 = 400.0;
/// No cop starts nearer than this to another car (`CheckSpawnPosition`'s 40 m for a spot that is not a traffic lane).
const MIN_GAP: f32 = blackbox_pursuit::spawn_rules::ANY_OTHER;
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
    /// The distance from `target` the spot must land in once it is on the road, metres, horizontally: the cop spawn
    /// window of `docs/specs/ai-pursuit-cops.md` section 3.2. `None` for the cars that have their own placement.
    pub band: Option<(f32, f32)>,
}

/// The chase: the heat rows of all levels, the heat it is at, and the spawn bookkeeping. The wave is the row of the
/// current heat, so a rise in heat brings the cop types of the higher row.
pub struct Pursuit {
    pub(super) damaged: std::collections::BTreeSet<u32>,
    /// Last observed target in physics space. Search never reads the hidden player's live pose.
    pub(super) last_seen: Option<blackbox_roads::Body>,
    pub heat: u32,
    /// The rows of the heat levels, index = level - 1 (`nfsmw_data::pursuit::heat_rows`).
    rows: Vec<HeatRow>,
    player: Performance,
    spawn_timer: f32,
    pub(super) involved: u32,
    /// The cops still to evade and the backup that refills them (`blackbox_pursuit::backup`).
    pub(super) budget: Budget,
    /// The backup values of every heat level, index = level - 1.
    backups: Vec<BackupRow>,
    /// The cops that have stood within the engagement radius and are still in the chase, by audio identity.
    pub(super) engaged: std::collections::HashSet<u32>,
    /// The chase's support request and the cars it took (`support`).
    pub(super) support: super::support::Session,
}

impl Pursuit {
    pub fn new(heat: u32, rows: Vec<HeatRow>, player: Performance) -> Self {
        let required = rows.get(heat.max(1) as usize - 1).or(rows.last()).map_or(0, |row| row.full_engagement_cops);
        Self {
            budget: Budget::new(required, BackupRow::default().trigger),
            backups: Vec::new(),
            engaged: Default::default(),
            heat,
            rows,
            player,
            last_seen: None,
            damaged: Default::default(),
            spawn_timer: 0.0,
            involved: 0,
            support: Default::default(),
        }
    }

    /// Gives the chase the backup values of the heat levels and restarts its budget from the row of the current heat.
    pub(super) fn set_backups(&mut self, backups: Vec<BackupRow>) {
        self.backups = backups;
        let required = self.row().map_or(0, |row| row.full_engagement_cops);
        self.budget.refill(required, self.backup().trigger);
    }

    /// The backup values of the heat the chase is at now.
    pub(super) fn backup(&self) -> BackupRow {
        let last = self.backups.len().checked_sub(1);
        let at = (self.heat.max(1) as usize - 1).min(last.unwrap_or(0));
        self.backups.get(at).copied().unwrap_or_default()
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

    /// The helicopter was picked: it counts among the cops involved and arms the next spawn delay like a car does.
    pub(super) fn sent_helicopter(&mut self) {
        let delay = self.row().map(|r| match self.involved < 3 {
            true => r.time_between_first_four_spawns,
            false => r.time_between_cop_spawns,
        });
        self.involved += 1;
        self.spawn_timer = delay.unwrap_or(0.0);
    }

    /// Follows the police heat level: a new level takes effect at once. Returns true when the level changed.
    pub fn follow_heat(&mut self, level: u32) -> bool {
        if level == self.heat {
            return false;
        }
        self.heat = level;
        true
    }

    /// Drops the wave's entries of the cop car `name`, whose model did not load: the wave does not wait on it. The rows
    /// of every level go, as `begin_pursuit` drops them (`cop_rows::without_models`).
    fn drop_model(&mut self, name: &str) {
        let rows = std::mem::take(&mut self.rows);
        self.rows = without_models(rows, &[name.to_owned()]);
    }

    /// The car types of the wave at the current heat (without the helicopter) with how many of each.
    pub fn wave(&self) -> impl Iterator<Item = (&str, u32, u32)> {
        self.row().into_iter().flat_map(ground_cops).map(|c| (c.name.as_str(), c.count, c.chance))
    }

    /// The fewest and the most cops the chase may have now. Cooling down it keeps exactly the row's patrol count; otherwise
    /// the most is the cops still to evade (`Budget::allowed`), held to the level's cap from the Bartender file and to the
    /// chase's cop limit (`GetAdjustedCopCounts`).
    fn limits(&self, settings: &BartenderSettings, cooldown: bool) -> (u32, u32) {
        let Some(row) = self.row() else { return (0, 0) };
        let limit = settings.max_cops;
        if cooldown {
            let patrol = row.patrol_cars.min(limit);
            return (patrol, patrol);
        }
        let most = match settings.cop_cap(self.heat) {
            Some(cap) => self.budget.allowed().min(cap.max(1)),
            None => self.budget.allowed(),
        };
        (0, most.min(limit))
    }

    /// How many cops the chase wants now: the wave's total held between the limits.
    pub fn wanted(&self, settings: &BartenderSettings, cooldown: bool) -> usize {
        let nominal: u32 = self.wave().map(|w| w.1).sum();
        let (least, most) = self.limits(settings, cooldown);
        want_cops(nominal, least, most) as usize
    }

    /// The wave scaled to the cops wanted now: each car type with the count the chase may have of it and its weight. A
    /// type scaled to nothing is left out.
    pub fn adjusted_wave(&self, settings: &BartenderSettings, cooldown: bool) -> Vec<(&str, u32, u32)> {
        let nominal: Vec<(&str, u32, u32)> = self.wave().collect();
        let counts: Vec<u32> = nominal.iter().map(|w| w.1).collect();
        let scaled = adjusted_counts(&counts, self.wanted(settings, cooldown) as u32);
        nominal
            .into_iter()
            .zip(scaled)
            .filter(|(_, n)| *n > 0)
            .map(|((name, _, chance), n)| (name, n, chance))
            .collect()
    }
}

impl TrafficWorld {
    /// Starts a chase at `heat` with the wave rows `rows` (one per level); `player` is how the player's car performs. The
    /// rows are the ones `ai/commands.rs` `begin_pursuit` prepared: the mods' waves applied, the cars of them loaded.
    /// The cars of the support requests are asked for too, at every level, so they load while the chase waits for its
    /// first support (a support car whose model is not in yet waits for it, `ai/support`).
    pub fn start_pursuit(&mut self, heat: u32, rows: Vec<HeatRow>, player: Performance) {
        self.cops.unlock();
        self.police.begin(heat);
        self.roadblocks.begin_chase();
        let mut pursuit = Pursuit::new(self.police.patrol_heat(), rows, player);
        pursuit.set_backups(self.backup_rows.clone());
        self.pursuit = Some(pursuit);
        let range = blackbox_pursuit::sight::ranges(self.police.patrol_heat()).0;
        for car in &mut self.cars {
            let near = self.player_position.is_some_and(|p| car.physics_position().distance(p) < range);
            if !near {
                continue;
            }
            let Some(model) = self.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case(&car.name)) else {
                continue;
            };
            if car.join_pursuit(model.car, player, &self.network, &self.index)
                && let Some(pursuit) = self.pursuit.as_mut()
            {
                pursuit.involved += 1;
            }
        }
        for name in super::support::support_models() {
            self.request_cop(name);
        }
    }

    /// The Blacklist's current bin while a career is on (`None` without one): the heat range and growth time of a chase.
    pub fn set_career_bin(&mut self, bin: Option<u8>) {
        self.police.set_career_bin(bin);
    }

    /// Seconds the cops are still locked out (no patrols, no spawns): 0 when they may appear.
    pub fn cop_lockout_left(&self) -> f32 {
        self.cops.lockout_left()
    }

    /// Keeps the cops out of the world (`true`) while a race with no police runs, ending a chase that is on; `false`
    /// lets the lockout rule again.
    pub fn bar_cops(&mut self, barred: bool) {
        self.cops.bar(barred);
        if barred && self.is_pursuing() {
            self.stop_pursuit();
        }
    }

    /// Ends the chase without a result (the `pursuit off` command) and removes the cops.
    pub fn stop_pursuit(&mut self) {
        self.end_pursuit(EndReason::None);
    }

    /// Ends the chase with `reason` and removes the cops. The reason stays readable after it (the end card, the camera).
    pub fn end_pursuit(&mut self, reason: EndReason) {
        self.pursuit = None;
        self.police.end(reason);
        if reason == EndReason::Evaded {
            self.cops.lock_out();
        }
        match reason == EndReason::Busted && self.arrest_active() {
            true => {
                for car in self.cars.iter_mut().filter(|car| car.is_police()) {
                    car.hold_for_arrest();
                }
            }
            // An escape leaves the roadblocks standing: their cars and props go by the rules that retire what nobody sees.
            false => self.cars.retain(|car| !car.is_police() || (reason == EndReason::Evaded && car.is_roadblock())),
        }
        self.seen_by_cops = false;
        if reason != EndReason::Evaded {
            self.clear_spikes();
        }
        self.roadblocks.end_chase();
    }

    pub fn add_cop_model(&mut self, model: CopModel) {
        self.known.insert(model.model.name.to_ascii_lowercase());
        self.cop_models.push(model);
    }

    pub fn has_cop_model(&self, name: &str) -> bool {
        self.cop_models.iter().any(|m| m.model.name.eq_ignore_ascii_case(name))
    }

    /// A cop car model did not load: a chase that is running drops the wave's entries of it (`Pursuit::drop_model`),
    /// and the support requests that take it (`ai/support`).
    pub(super) fn cop_model_failed(&mut self, name: &str) {
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        pursuit.drop_model(name);
        pursuit.support.model_failed(name);
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
        if !self.police.can_begin() {
            return;
        }
        match self.police.is_perp_in_sight() {
            true => self.update_support(sim_dt, focus, collision, surfaces),
            false => self.age_search_support(sim_dt),
        }
        let level = self.police.level();
        let cooldown = self.police.cooldown_status() == blackbox_pursuit::CooldownStatus::CoolDown;
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        if let Some(level) = level
            && pursuit.follow_heat(level)
        {
            let wave: Vec<&str> = pursuit.wave().map(|w| w.0).collect();
            log::info!("pursuit: heat {level}, the wave is now {}", wave.join(", "));
        }
        pursuit.spawn_timer -= sim_dt;
        self.update_budget(sim_dt, focus);
        self.flee_surplus(focus, cooldown);
        // A latched roadblock request stops the manager from spawning pursuit cops until the roadblock stands.
        if self.roadblocks.dispatcher_latched() {
            return;
        }
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        let alive = self.cars.iter().filter(|c| c.is_cop()).count();
        let wanted = pursuit.wanted(self.police.settings(), cooldown);
        if pursuit.spawn_timer > 0.0 {
            return;
        }
        // The helicopter takes its turn among the wave's types (`ai/air.rs`); a pick counts as a cop spawned.
        if self.pick_helicopter(&player, alive < wanted) || alive >= wanted {
            return;
        }
        // The manager asks for a wave cop only below the active-car budget (`mNumActiveCopCars < mMaxActiveCopCars`).
        if !self.budget_has_room() {
            return;
        }
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        // The next type of the current wave, weighted by its chance among those the wave still needs.
        let pick = {
            let adjusted = pursuit.adjusted_wave(self.police.settings(), cooldown);
            let slots: Vec<Slot> = adjusted.iter().map(|&(name, count, chance)| Slot { name, count, chance }).collect();
            let have =
                |name: &str| self.cars.iter().filter(|c| c.is_cop() && c.name.eq_ignore_ascii_case(name)).count();
            let roll = self.rng.police.next_f32();
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
        let spawned = match self.police.is_perp_in_sight() {
            true => self.spawn_cop(&name, focus, player.position, performance, collision, surfaces),
            false => self.spawn_search_cop(&name, focus, performance, collision, surfaces),
        };
        if spawned && let Some(p) = self.pursuit.as_mut() {
            p.involved += 1;
            p.spawn_timer = delay;
        }
    }

    /// Puts a cop car of `name` on the cop roads 150 to 400 m from the camera, facing the player. The spot comes from the
    /// walk from the player's seek-ahead point; when the roads or the checks give none, from the disc 200 m ahead
    /// (`seek_spawn.rs`). Nothing is placed, and the timer is not reset, when neither passes.
    fn spawn_cop(
        &mut self,
        name: &str,
        focus: Focus,
        target: Vec3,
        player: Performance,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let Some(body) = focus.body else { return false };
        let band = Some((MIN_DISTANCE, MAX_DISTANCE));
        for at in self.chase_spawn_points(&body) {
            let spot = CopSpot { at, target, speed: START_SPEED, player, band };
            if self.spawn_cop_at(name, spot, collision, surfaces) {
                return true;
            }
        }
        false
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
        let CopSpot { at: physics_at, target, speed, player, band } = spot;
        let Some(model) = self.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case(name)) else {
            return false;
        };
        let toward = Vec3::new(target.x - physics_at.x, 0.0, target.z - physics_at.z);
        let filter = SegmentFilter { cop: true, no_decision: true, ..SegmentFilter::default() };
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, filter);
        if !nav.init_at_point(&self.network, &self.index, physics_at, toward, false) {
            return false;
        }
        if self.cops.bars_spawn(nav.position) {
            return false;
        }
        // The window is measured from the camera (`AISpawnManager::GetBasePosition`), the player until the world gives it.
        let gap = self.cops.window_gap(nav.position, target);
        if band.is_some_and(|(near, far)| !(near..=far).contains(&gap)) {
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
        let start = Start {
            stagger: (self.rng.police.next_f32() * 8.0) as u32,
            speed,
            cruise: Cruise::default(),
            patrol: false,
        };
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
        log::info!("pursuit: {} at ({:.0}, {:.0}), {:.0} m from the camera", car.name, render.x, render.y, gap);
        self.admit(car);
        true
    }

    /// One line for the `pursuit status` command.
    pub fn pursuit_status(&self) -> String {
        let Some(p) = &self.pursuit else {
            let lockout = match self.cop_lockout_left() {
                left if left > 0.0 => format!("no pursuit; the cops are locked out for {left:.0} s"),
                _ => "no pursuit".into(),
            };
            return format!("{lockout}; {}; {}", self.call_911_status(), self.fleet_status());
        };
        let cops: Vec<String> = self.cars.iter().filter(|c| c.is_cop()).map(|c| c.name.clone()).collect();
        let infractions: Vec<String> = self.police.infractions().iter().map(|i| format!("{i:?}")).collect();
        let formation = self.police.last_formation().map_or("none yet".to_owned(), |f| format!("{f:?}"));
        let to_evade = self.pursuit_feed().map_or(0.0, |feed| feed.secs_to_evade);
        let heat = self.police.heat_value();
        let seconds_per_level = self.police.settings().time_per_level(p.heat);
        let roadblocks = self.roadblocks.stats();
        let helicopter = self.helicopter_status().map_or("no helicopter".to_owned(), |h| {
            format!(
                "helicopter {:?}, {:.0} s of fuel{}",
                h.mode,
                h.fuel_secs,
                if h.in_sight { ", sees you" } else { "" }
            )
        });
        format!(
            "heat {heat:.3} ({seconds_per_level:.0} s/level outside cooldown): {} of {} cops ({}); {} to evade{}; formation {formation}; roadblocks {} up, {} built, {} dodged; {helicopter}; evade in {to_evade:.0} s; infractions {} for ${}{}; {}",
            cops.len(),
            p.wanted(
                self.police.settings(),
                self.police.cooldown_status() == blackbox_pursuit::CooldownStatus::CoolDown
            ),
            cops.join(", "),
            p.budget.remaining(),
            p.budget.eta().map_or(String::new(), |eta| format!(", backup in {eta:.0} s")),
            self.roadblocks.count(),
            roadblocks.deployed,
            roadblocks.dodged,
            infractions.len(),
            self.police.infractions().total_fines(),
            if infractions.is_empty() { String::new() } else { format!(" ({})", infractions.join(", ")) },
            self.fleet_status(),
        )
    }
}

#[cfg(test)]
pub(super) mod tests;

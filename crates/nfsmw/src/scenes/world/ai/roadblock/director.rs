//! The cop manager's roadblock work, once a frame (`UpdatePursuits` and `UpdateRoadBlocks`, spec sections 1 and 5): the
//! lottery that asks for a roadblock and the latch that builds it, the heavy support request, and for each standing
//! roadblock the dodge, the "roadblock sees you" rule, the cheat rule that recruits a car, and the release when no car
//! is left. Repeated roadblocks follow from the release: the pursuit asks again once its roadblock is gone.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::roadblock::request::{Ask, odds};
use blackbox_pursuit::roadblock::standing::{self, Car, SEES_YOU};
use blackbox_pursuit::support::Strategy;
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::build::Cause;
use super::state::HeavyRequest;
use crate::scenes::world::ai::support;
use crate::scenes::world::ai::{PlayerView, TrafficWorld};
use crate::scenes::world::props::PropWorld;
use crate::scenes::world::space;

/// The cop manager runs its roadblock logic this often, seconds.
const RUN_PERIOD: f32 = 0.5;
/// A heavy roadblock request that finds no site for this long is dropped, seconds (the original retries without end;
/// a chase that cannot place one should not hold the request for ever).
const HEAVY_PATIENCE: f32 = 60.0;

impl TrafficWorld {
    /// Advances the roadblocks by `dt` seconds of simulated time. `props` is the prop world (the sawhorses live there).
    pub(in crate::scenes::world) fn update_roadblocks(
        &mut self,
        dt: f32,
        view: PlayerView,
        collision: &CollisionWorld,
        props: Option<&mut PropWorld>,
        surfaces: &SurfaceTable,
    ) {
        let target = Vec3::from_array(space::to_physics(view.position));
        let pursuing = self.is_pursuing();
        self.roadblocks.dispatcher.tick(dt);
        if self.police.take_roadblock_request() {
            let level = self.police.level().unwrap_or(1);
            let big_suv = support::big_suv_chance(level, Strategy::HeavyRoadblock);
            self.roadblocks.heavy = Some(HeavyRequest { big_suv, waited: 0.0 });
            self.roadblocks.dispatcher.heavy_chosen();
        }
        self.roadblocks_watch(dt, target, pursuing);
        self.roadblocks.clock += dt;
        if self.roadblocks.clock >= RUN_PERIOD {
            let step = self.roadblocks.clock;
            self.roadblocks.clock = 0.0;
            self.roadblocks_release();
            if pursuing {
                self.roadblocks_request(step, view, collision, props, surfaces);
            }
        }
        self.police.set_roadblock_standing(self.roadblocks.any());
        self.police.set_roadblock_follow(self.roadblocks.forces_follow());
    }

    /// The live cars of roadblock `id` as the distance rules see them.
    fn roadblock_cars(&self, id: u32) -> Vec<Car> {
        self.cars
            .iter()
            .filter(|car| car.roadblock_group() == Some(id) && !car.is_wrecked())
            .map(|car| Car { id: car.key() as usize, position: car.physics_position() })
            .collect()
    }

    /// Every frame: the roadblock's damage counts, the dodge, the distance to the target and the cheat timer.
    fn roadblocks_watch(&mut self, dt: f32, target: Vec3, pursuing: bool) {
        let cooling = self.police.cooldown_status() == blackbox_pursuit::CooldownStatus::CoolDown;
        let hidden = self.hidden_from_cars();
        let mut sees = false;
        let mut recruit = Vec::new();
        for index in 0..self.roadblocks.list.len() {
            let id = self.roadblocks.list[index].id;
            let (damaged, wrecked) = self.cars.iter().filter(|car| car.roadblock_group() == Some(id)).fold(
                (0, 0),
                |(damaged, wrecked), car| {
                    (damaged + u32::from(car.cop_damage() > 0.0), wrecked + u32::from(car.is_wrecked()))
                },
            );
            let cars = self.roadblock_cars(id);
            let reach = standing::reach(cars.iter().copied(), target);
            let block = &mut self.roadblocks.list[index];
            block.standing.damaged = block.standing.damaged.max(damaged);
            block.standing.destroyed = block.standing.destroyed.max(wrecked);
            if !pursuing {
                continue;
            }
            // A roadblock sees the player within 60 m unless a hiding spot hides it from cars.
            sees |= !hidden && reach.min < SEES_YOU;
            if let Some(dodged) = block.standing.dodge(target) {
                self.roadblocks.stats.dodged += 1;
                self.note_speech(crate::audio::WorldEvent::RoadblockDodged);
                self.roadblocks.stats.spike_strips_dodged += dodged.spike_strips;
                log::info!("roadblock {id}: dodged ({} spike strips)", dodged.spike_strips);
            }
            let block = &mut self.roadblocks.list[index];
            block.standing.watch(dt, reach.min_xz, target, cooling);
            if block.standing.is_cheating() && block.recruited == 0 {
                recruit.extend(reach.nearest.map(|key| (id, key as u32)));
            }
        }
        self.roadblocks.sees_player = sees;
        for (id, key) in recruit {
            self.recruit_from_roadblock(id, key);
        }
    }

    /// The pursuit takes one car of the roadblock into its own list (`AIPursuit` when the roadblock reports the perp
    /// cheating): the nearest one, once. It becomes an ordinary pursuit cop.
    fn recruit_from_roadblock(&mut self, id: u32, key: u32) {
        let Some(pursuit) = self.pursuit.as_ref() else { return };
        let performance = pursuit.performance();
        let Some(index) = self.cars.iter().position(|car| car.key() == key && car.roadblock_group() == Some(id)) else {
            return;
        };
        let Some(model) = self.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case(&self.cars[index].name))
        else {
            return;
        };
        let cop = model.car;
        if !self.cars[index].recruit_to_pursuit(cop, performance, &self.network, &self.index) {
            return;
        }
        if let Some(block) = self.roadblocks.list.iter_mut().find(|block| block.id == id) {
            block.recruited += 1;
        }
        self.roadblocks.stats.recruited += 1;
        log::info!("roadblock {id}: a car joins the chase (the perp was cheating)");
    }

    /// A roadblock with no live car left is released: its props go with it (`UpdateRoadBlocks`).
    fn roadblocks_release(&mut self) {
        let cars = &self.cars;
        self.roadblocks.list.retain(|block| {
            let alive = cars.iter().any(|car| car.roadblock_group() == Some(block.id) && !car.is_wrecked());
            if !alive {
                log::info!("roadblock {}: released", block.id);
            }
            alive
        });
    }

    /// The manager's run: the heavy support request first, then the lottery (`UpdateSupportCops`, `UpdatePursuits`).
    fn roadblocks_request(
        &mut self,
        step: f32,
        view: PlayerView,
        collision: &CollisionWorld,
        mut props: Option<&mut PropWorld>,
        surfaces: &SurfaceTable,
    ) {
        if let Some(mut heavy) = self.roadblocks.heavy.take() {
            let built = match self.roadblocks.any() {
                true => None,
                false => Some(self.build_roadblock(
                    Cause::Heavy { big_suv: heavy.big_suv },
                    view,
                    collision,
                    props.as_deref_mut(),
                    surfaces,
                )),
            };
            heavy.waited += step;
            match built {
                Some(Ok(cars)) => {
                    log::info!("support: a heavy roadblock of {cars} cop cars");
                    self.police.heavy_roadblock_built();
                }
                Some(Err(e)) if heavy.waited < HEAVY_PATIENCE => {
                    log::debug!("support: no heavy roadblock yet: {e}");
                    self.roadblocks.heavy = Some(heavy);
                }
                Some(Err(e)) => log::warn!("support: the heavy roadblock was dropped: {e}"),
                None => {}
            }
        }
        let level = self.police.level().unwrap_or(1);
        let last_known = self
            .pursuit
            .as_ref()
            .and_then(|pursuit| pursuit.last_seen)
            .map_or(0.0, |seen| seen.position.distance(Vec3::from_array(space::to_physics(view.position))));
        let ask = Ask {
            busted: self.police.is_busted(),
            bailed: false,
            roadblock_exists: self.roadblocks.any(),
            pursuit_time: self.police.pursuit_secs(),
            min_delay: support::minimum_delay(level),
            in_sight: self.police.is_perp_in_sight(),
            last_known_distance: last_known,
        };
        let rolls = [self.rng_unit(), self.rng_unit()];
        if !self.roadblocks.dispatcher.poll(&ask, &odds(level), rolls) {
            return;
        }
        match self.build_roadblock(Cause::Lottery, view, collision, props, surfaces) {
            Ok(cars) => {
                log::info!("roadblock request met: {cars} cop cars");
                self.roadblocks.dispatcher.built();
            }
            Err(e) => log::debug!("roadblock request waits: {e}"),
        }
    }

    /// A uniform number in `0..1` from the world's generator.
    fn rng_unit(&mut self) -> f32 {
        use blackbox_roads::RandomSource;
        self.rng.roadblock.next_f32()
    }
}

//! The support request of a chase, run each tick (spec `ai-pursuit-support.md` sections 3 to 7 and 11; formations spec
//! section 6). A request waits pending while its cars are taken (the ram delay arms per ram car), then its cars are placed
//! one by one at their points and it turns active for the option's duration. Its cars are released when the timer runs out
//! or when none of them is left. A car whose model is still loading holds the request pending: the request waits and
//! tries again on a later step. A car whose model failed to load ends the request, and its strategy is refused for the
//! rest of the chase. The roll that makes the request is outside this file.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::support::cars::car_for;
use blackbox_pursuit::support::{Car, Order, RamDelay, Request, Search, Strategy};
use blackbox_roads::RandomSource;
use blackbox_vehicle::performance::Performance;
use nfsmw_data::car::physics::SurfaceTable;

use super::super::{Focus, TrafficWorld};
use super::car_model;
use super::spawn::SupportSpot;

/// One chase's support request and the bookkeeping of its cars.
#[derive(Debug, Default)]
pub struct Session {
    order: Option<Order>,
    /// Counts the requests of this chase; a car's tag names the request it belongs to.
    serial: u32,
    /// The car model of each car the request has taken, in the order taken.
    cars: Vec<Car>,
    /// How many of `cars` are placed.
    placed: u32,
    /// The try counter of the placement search, kept across the request's cars.
    search: Search,
    /// The shared ram delay (the manager's timer in the binary).
    delay: RamDelay,
    /// The cop car models of this chase that did not load (names as the loader reports them).
    failed: Vec<String>,
    /// The strategies this chase refuses: a request of one of them took a car whose model did not load.
    dropped: Vec<Strategy>,
}

impl Session {
    pub(in crate::scenes::world::ai) fn is_busy(&self) -> bool {
        self.order.is_some()
    }

    /// Starts a request from the roll, when no request of this chase is pending or active. A heavy roadblock is not
    /// taken here: the roadblock is built by its own code (`ai/roadblock.rs`). A strategy the chase dropped is refused.
    /// Returns whether it was taken.
    pub fn request(&mut self, request: Request) -> bool {
        if self.order.is_some() || request.strategy == Strategy::HeavyRoadblock {
            return false;
        }
        if self.dropped.contains(&request.strategy) {
            return false;
        }
        self.serial += 1;
        self.cars.clear();
        self.placed = 0;
        self.search = Search::default();
        self.order = Some(Order::new(request));
        true
    }

    /// The pending or active request of the chase, if there is one.
    pub(in crate::scenes::world::ai) fn order(&self) -> Option<&Order> {
        self.order.as_ref()
    }

    /// A cop model did not load: a request that takes its car is dropped (`takes_failed_model`).
    pub fn model_failed(&mut self, name: &str) {
        if self.failed.iter().any(|known| known.eq_ignore_ascii_case(name)) {
            return;
        }
        self.failed.push(name.to_owned());
    }

    /// Whether a car the request has taken is a model that did not load.
    fn takes_failed_model(&self) -> bool {
        self.cars.iter().any(|car| self.failed.iter().any(|name| name.eq_ignore_ascii_case(car_model(*car))))
    }

    /// Ends the pending request: its strategy is refused for the rest of the chase, and the request forgets its cars.
    fn drop_order(&mut self) {
        if let Some(order) = self.order.take() {
            self.dropped.push(order.strategy());
        }
        self.cars.clear();
        self.placed = 0;
    }
}

impl TrafficWorld {
    /// Active heavy support leaves on loss of contact or collapse; pending retries and leaders are separate.
    pub(in crate::scenes::world::ai) fn cancel_heavy_support(&mut self) {
        let Some(session) = self.pursuit.as_mut().map(|p| &mut p.support) else { return };
        let Some(order) = session.order else { return };
        if !order.is_active() || !order.strategy().is_heavy() {
            return;
        }
        let serial = session.serial;
        session.order = None;
        session.cars.clear();
        session.placed = 0;
        let cruise = self.search_cruise();
        for car in self.cars.iter_mut().filter(|car| car.support_serial() == Some(serial)) {
            car.set_support(None);
            car.start_flee(cruise, &self.network, &self.index);
        }
    }

    /// Pending and active support timers continue during search; pending offensive cars cannot spawn.
    pub(in crate::scenes::world::ai) fn age_search_support(&mut self, dt: f32) {
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        let session = &mut pursuit.support;
        session.delay.tick(dt);
        let Some(mut order) = session.order else { return };
        if !order.tick(dt) {
            session.order = Some(order);
            return;
        }
        let serial = session.serial;
        session.order = None;
        session.cars.clear();
        session.placed = 0;
        self.release_support(serial);
    }

    /// Hands a request from the roll to the chase. Returns whether it was taken (no request of the chase is under way).
    pub fn ask_support(&mut self, request: Request) -> bool {
        let taken = self.pursuit.as_mut().is_some_and(|pursuit| pursuit.support.request(request));
        if taken && request.strategy.is_heavy() {
            self.roadblocks.delay_for_heavy_support();
        }
        taken
    }

    /// Runs the chase's support request for one step: the cars are taken and placed, and the request times out. A ram or
    /// leader request the roll made since the last step is taken first.
    pub(in crate::scenes::world::ai) fn update_support(
        &mut self,
        dt: f32,
        focus: Focus,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) {
        if let Some(request) = self.police.take_support_request() {
            self.ask_support(request);
        }
        let Some(target) = focus.body else { return };
        let Some(pursuit) = self.pursuit.as_mut() else { return };
        let performance = pursuit.performance();
        let mut session = std::mem::take(&mut pursuit.support);
        let level = self.police.level().unwrap_or(1);
        let step = Step { dt, target, performance, level };
        self.run_support(&mut session, step, collision, surfaces);
        if let Some(pursuit) = self.pursuit.as_mut() {
            pursuit.support = session;
        }
    }

    /// The request's step: the timer while active, else the acquisition and the placement of its cars.
    fn run_support(&mut self, session: &mut Session, step: Step, collision: &CollisionWorld, surfaces: &SurfaceTable) {
        session.delay.tick(step.dt);
        let Some(mut order) = session.order else { return };
        if order.tick(step.dt) {
            if order.is_pending() {
                log::warn!(
                    "support: {:?} expired before activation: {}/{} cars placed, {} placement denials",
                    order.strategy(),
                    session.placed,
                    order.cars_wanted(),
                    order.denials()
                );
            }
            let serial = session.serial;
            session.order = None;
            session.cars.clear();
            session.placed = 0;
            self.release_support(serial);
            return;
        }
        if order.is_active() {
            let serial = session.serial;
            let lost = session.placed > 0 && !self.cars.iter().any(|c| c.support_serial() == Some(serial));
            session.order = match lost {
                true => None,
                false => Some(order),
            };
            if session.order.is_none() {
                self.release_support(serial);
            }
            return;
        }
        if self.world_moment {
            session.order = Some(order);
            return;
        }
        let strategy = order.strategy();
        let wanted = order.cars_wanted() as usize;
        let big_suv = self.police.big_suv_chance(step.level, strategy);
        let taking = session.cars.len() < wanted;
        while session.cars.len() < wanted {
            let index = session.cars.len() as u32;
            let rand = self.rng.police.next_f32();
            session.cars.push(car_for(strategy, index, big_suv, rand));
            if strategy == Strategy::Ram {
                session.delay.arm();
            }
        }
        // The cars of a ram request are taken: the radio hears the call for the Rhino units.
        if taking && strategy == Strategy::Ram {
            self.note_speech(crate::audio::WorldEvent::BackupRequested(blackbox_pursuit::speech::Backup::Rhino));
        }
        if session.takes_failed_model() {
            self.drop_request(session);
            return;
        }
        if session.delay.is_clear() {
            self.place_support(session, &mut order, step, collision, surfaces);
        }
        session.order = Some(order);
    }

    /// Ends a pending request whose car did not load: the cars it already placed go back to the chase, and its strategy
    /// is refused for the rest of the chase (the other strategies are not affected).
    fn drop_request(&mut self, session: &mut Session) {
        let serial = session.serial;
        self.release_support(serial);
        session.drop_order();
    }

    /// Places the cars of a pending request that are not yet placed. A refused spot is counted and the rest wait for the
    /// next step; when every car stands, the request turns active. A car whose model is still loading is not tried: the
    /// request waits for it, with no denial and no placement search used up.
    fn place_support(
        &mut self,
        session: &mut Session,
        order: &mut Order,
        step: Step,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) {
        let strategy = order.strategy();
        let distance: fn(u32) -> f32 = match strategy.is_heavy() {
            true => Search::heavy_distance,
            false => Search::leader_distance,
        };
        let serial = session.serial;
        while (session.placed as usize) < session.cars.len() {
            let index = session.placed;
            let car = session.cars[index as usize];
            if !self.has_cop_model(car_model(car)) {
                self.request_cop(car_model(car));
                return;
            }
            let placed = session.search.place(distance, |at| {
                let spot = SupportSpot {
                    car,
                    strategy,
                    index,
                    serial,
                    distance: at,
                    target: step.target,
                    performance: step.performance,
                };
                self.spawn_support_car(&spot, collision, surfaces).then_some(())
            });
            if placed.is_none() {
                order.deny();
                self.note_speech(crate::audio::WorldEvent::BackupDenied(backup_type(strategy)));
                return;
            }
            session.placed += 1;
        }
        if session.placed as usize == order.cars_wanted() as usize && order.activate() && !strategy.is_heavy() {
            self.note_speech(crate::audio::WorldEvent::BackupRequested(backup_type(strategy)));
        }
    }

    /// Returns the cars of request `serial` to the chase: their support tag goes, and they are cops like the others.
    fn release_support(&mut self, serial: u32) {
        for car in self.cars.iter_mut().filter(|c| c.support_serial() == Some(serial)) {
            car.set_support(None);
        }
    }
}

/// What the radio calls the backup of a strategy: the heavy cars are the Rhino units, the leader's cars the super pursuit
/// ones (the binary's backup types 16 and 64).
fn backup_type(strategy: Strategy) -> blackbox_pursuit::speech::Backup {
    use blackbox_pursuit::speech::Backup;
    match strategy.is_heavy() {
        true => Backup::Rhino,
        false => Backup::SuperPursuit,
    }
}

/// What one step of the request needs from the world: the time, the target and its performance, and the heat level.
#[derive(Debug, Clone, Copy)]
struct Step {
    dt: f32,
    target: blackbox_roads::Body,
    performance: Performance,
    level: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(strategy: Strategy) -> Request {
        Request { strategy, duration: 30.0 }
    }

    #[test]
    fn one_request_at_a_time() {
        let mut session = Session::default();
        assert!(session.request(request(Strategy::Ram)));
        assert!(!session.request(request(Strategy::EBrake)), "a request is already pending");
        assert_eq!(session.serial, 1);
    }

    #[test]
    fn a_heavy_roadblock_is_not_taken_here() {
        let mut session = Session::default();
        assert!(!session.request(request(Strategy::HeavyRoadblock)));
        assert!(session.order.is_none());
    }

    #[test]
    fn each_request_gets_a_new_serial() {
        let mut session = Session::default();
        session.request(request(Strategy::Ram));
        session.order = None;
        session.request(request(Strategy::CrossFollow));
        assert_eq!(session.serial, 2);
        assert_eq!(session.placed, 0);
    }

    #[test]
    fn a_request_whose_car_model_failed_is_dropped_and_its_strategy_refused_for_the_chase() {
        let mut session = Session::default();
        session.model_failed("COPCROSS");
        assert!(session.request(request(Strategy::CrossFollow)));
        session.cars.push(Car::Cross);
        assert!(session.takes_failed_model());
        session.drop_order();
        assert!(session.order().is_none());
        assert!(!session.request(request(Strategy::CrossFollow)), "the strategy is refused for the chase");
        assert!(session.request(request(Strategy::EBrake)), "other support is still taken");
    }

    #[test]
    fn a_car_whose_model_only_loads_later_is_not_a_failed_model() {
        let mut session = Session::default();
        assert!(session.request(request(Strategy::CrossFollow)));
        session.cars.push(Car::Cross);
        assert!(!session.takes_failed_model(), "a model still loading is not a failure");
        assert!(session.order().is_some(), "the request stays pending");
    }
}

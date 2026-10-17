//! The support request of a chase, run each tick (spec `ai-pursuit-support.md` sections 3 to 7 and 11; formations spec
//! section 6). A request waits pending while its cars are taken (the ram delay arms per ram car), then its cars are placed
//! one by one at their points and it turns active for the option's duration. Its cars are released when the timer runs out
//! or when none of them is left. The roll that makes the request is outside this file.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::support::cars::car_for;
use blackbox_pursuit::support::{Car, Order, RamDelay, Request, Search, Strategy};
use blackbox_roads::RandomSource;
use blackbox_vehicle::performance::Performance;
use nfsmw_data::car::physics::SurfaceTable;

use super::super::{Focus, TrafficWorld};
use super::big_suv_chance;
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
}

impl Session {
    /// Starts a request from the roll, when no request of this chase is pending or active. A heavy roadblock is not taken
    /// here: the roadblock is built by its own code (`ai/roadblock.rs`). Returns whether it was taken.
    pub fn request(&mut self, request: Request) -> bool {
        if self.order.is_some() || request.strategy == Strategy::HeavyRoadblock {
            return false;
        }
        self.serial += 1;
        self.cars.clear();
        self.placed = 0;
        self.search = Search::default();
        self.order = Some(Order::new(request));
        true
    }
}

impl TrafficWorld {
    /// Hands a request from the roll to the chase. Returns whether it was taken (no request of the chase is under way).
    pub fn ask_support(&mut self, request: Request) -> bool {
        self.pursuit.as_mut().is_some_and(|pursuit| pursuit.support.request(request))
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
        if order.is_active() {
            let ended = order.tick(step.dt);
            let serial = session.serial;
            let lost = session.placed > 0 && !self.cars.iter().any(|c| c.support_serial() == Some(serial));
            session.order = match ended || lost {
                true => None,
                false => Some(order),
            };
            if session.order.is_none() {
                self.release_support(serial);
            }
            return;
        }
        let strategy = order.strategy();
        let wanted = order.cars_wanted() as usize;
        let big_suv = big_suv_chance(step.level, strategy);
        while session.cars.len() < wanted {
            let index = session.cars.len() as u32;
            let rand = self.rng.next_f32();
            session.cars.push(car_for(strategy, index, big_suv, rand));
            if strategy == Strategy::Ram {
                session.delay.arm();
            }
        }
        if session.delay.is_clear() {
            self.place_support(session, &mut order, step, collision, surfaces);
        }
        session.order = Some(order);
    }

    /// Places the cars of a pending request that are not yet placed. A refused spot is counted and the rest wait for the
    /// next step; when every car stands, the request turns active.
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
                return;
            }
            session.placed += 1;
        }
        if session.placed as usize == order.cars_wanted() as usize {
            order.activate();
        }
    }

    /// Returns the cars of request `serial` to the chase: their support tag goes, and they are cops like the others.
    fn release_support(&mut self, serial: u32) {
        for car in self.cars.iter_mut().filter(|c| c.support_serial() == Some(serial)) {
            car.set_support(None);
        }
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
}

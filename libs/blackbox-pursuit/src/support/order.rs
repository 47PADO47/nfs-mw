//! The state of one support request (`UpdateSupportCops`, spec `ai-pursuit-support.md` section 3): pending while its cars
//! are taken and placed, active for the option's duration once they stand, then over. A placement that fails leaves the
//! request pending and is counted, and the next tick tries again (the binary sends `ReqDenied` on each try).

use super::cars::cars_for;
use super::{Request, Strategy};

/// Where the request is. The binary's values: 1 pending, 2 active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Pending,
    Active,
}

/// A request from the roll: its strategy, its car count and its state, with the timer that runs while it is active.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Order {
    strategy: Strategy,
    wanted: u32,
    duration: f32,
    state: State,
    timer: f32,
    denials: u32,
}

impl Order {
    /// A new pending request for `request`'s strategy, with the option's duration for later.
    pub fn new(request: Request) -> Self {
        Self {
            strategy: request.strategy,
            wanted: cars_for(request.strategy),
            duration: request.duration,
            state: State::Pending,
            timer: 0.0,
            denials: 0,
        }
    }

    pub fn strategy(&self) -> Strategy {
        self.strategy
    }

    /// How many cars the request takes in all.
    pub fn cars_wanted(&self) -> u32 {
        self.wanted
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn is_pending(&self) -> bool {
        self.state == State::Pending
    }

    pub fn is_active(&self) -> bool {
        self.state == State::Active
    }

    /// The seconds left of an active request (the option's duration at the start).
    pub fn timer(&self) -> f32 {
        self.timer
    }

    /// How many placements were refused so far.
    pub fn denials(&self) -> u32 {
        self.denials
    }

    /// A placement was refused: the request stays pending and the next tick tries again.
    pub fn deny(&mut self) {
        self.denials += 1;
    }

    /// Every car stands: pending becomes active with the timer at the option's duration. Returns false, and changes
    /// nothing, when the request is not pending.
    pub fn activate(&mut self) -> bool {
        if !self.is_pending() {
            return false;
        }
        self.state = State::Active;
        self.timer = self.duration;
        true
    }

    /// Counts an active request down by `dt` seconds. Returns true on the tick it runs out: the request is then over and
    /// its cars go back to the chase (the reset of spec section 6). A pending request does not count.
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.is_active() {
            return false;
        }
        let before = self.timer;
        self.timer -= dt;
        before > 0.0 && self.timer <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(strategy: Strategy, duration: f32) -> Request {
        Request { strategy, duration }
    }

    #[test]
    fn a_new_request_is_pending_with_the_spec_car_count() {
        let order = Order::new(request(Strategy::Ram, 30.0));
        assert!(order.is_pending());
        assert!(!order.is_active());
        assert_eq!(order.cars_wanted(), 2);
        assert_eq!(order.denials(), 0);
    }

    #[test]
    fn a_refused_placement_stays_pending_and_is_counted() {
        let mut order = Order::new(request(Strategy::Ram, 30.0));
        order.deny();
        order.deny();
        assert!(order.is_pending());
        assert_eq!(order.denials(), 2);
    }

    #[test]
    fn activating_starts_the_option_duration() {
        let mut order = Order::new(request(Strategy::HeavyRoadblock, 40.0));
        assert!(order.activate());
        assert!(order.is_active());
        assert_eq!(order.timer(), 40.0);
    }

    #[test]
    fn only_a_pending_request_activates() {
        let mut order = Order::new(request(Strategy::EBrake, 20.0));
        assert!(order.activate());
        assert!(!order.activate(), "already active");
        assert_eq!(order.timer(), 20.0, "the timer is not restarted");
    }

    #[test]
    fn the_timer_runs_only_while_active_and_ends_once() {
        let mut order = Order::new(request(Strategy::CrossFollow, 1.0));
        assert!(!order.tick(5.0), "a pending request does not count down");
        assert_eq!(order.timer(), 0.0);
        order.activate();
        assert!(!order.tick(0.4));
        assert!(!order.tick(0.4));
        assert!(order.tick(0.4), "the tick that takes it to zero or below ends it");
        assert!(order.timer() <= 0.0);
    }

    #[test]
    fn a_long_tick_ends_the_request_in_one_step() {
        let mut order = Order::new(request(Strategy::Ram, 2.0));
        order.activate();
        assert!(order.tick(10.0));
    }
}

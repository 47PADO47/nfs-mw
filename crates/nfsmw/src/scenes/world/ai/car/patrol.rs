//! Transition a roaming unit into the chase without replacing its physical car.

use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, RoadNetwork, SegmentFilter, SegmentIndex};
use blackbox_vehicle::performance::Performance;

use super::*;

impl AiCar {
    pub(in crate::scenes::world::ai) fn join_pursuit(
        &mut self,
        car: cop::CopCar,
        player: Performance,
        net: &RoadNetwork,
        index: &SegmentIndex,
    ) -> bool {
        if !self.patrol || self.is_wrecked() {
            return false;
        }
        let mut nav = RoadNav::new(
            NavKind::Direction,
            LaneType::Cop,
            SegmentFilter { cop: true, no_decision: true, ..Default::default() },
        );
        nav.half_width = self.half.0;
        if !nav.init_at_point(net, index, self.physics_position(), self.physics_forward(), false) {
            return false;
        }
        nav.enable_trail(net);
        self.nav = nav;
        self.role = Role::Cop(Box::new(cop::CopState::new(car, player, self.speed().abs())));
        self.driver = Driver::new(ControllerKind::Pid, false);
        self.patrol = false;
        self.waiting = false;
        self.stop_memory = None;
        self.arrived = false;
        self.accident = Accident::None;
        self.goal.set(GoalDrive::default());
        true
    }

    /// Live police of any ground role can report a sighting or appear on radar.
    pub fn is_live_police(&self) -> bool {
        self.is_police() && !self.is_wrecked()
    }

    pub fn is_police(&self) -> bool {
        self.patrol || self.roadblock || matches!(self.role, Role::Cop(_))
    }

    pub fn emergency_lights(&self) -> bool {
        (self.is_cop() || self.roadblock) && !self.is_wrecked() && !self.arrested
    }

    pub fn hold_for_arrest(&mut self) {
        self.arrested = true;
        self.arrived = false;
        self.goal.set(GoalDrive::default());
    }

    pub fn is_searching(&self) -> bool {
        self.searching
    }

    /// Entering a hiding zone while watched does not erase sight; losing sight there latches per cop until exit.
    pub(in crate::scenes::world::ai) fn report_sight(&mut self, hidden: bool, visible: bool) -> bool {
        if hidden && self.perp_hidden_from_me {
            return false;
        }
        self.perp_hidden_from_me = hidden && !visible;
        visible
    }

    /// Switch actions without moving or replacing the physical vehicle.
    pub(in crate::scenes::world::ai) fn search_mode(
        &mut self,
        searching: bool,
        cruise: Cruise,
        net: &RoadNetwork,
        index: &SegmentIndex,
        rng: &mut impl RandomSource,
    ) {
        if !self.is_cop() || self.roadblock || self.support.is_some() || self.searching == searching || self.arrested {
            return;
        }
        let (kind, lane, filter) = match searching {
            true => (NavKind::Traffic, LaneType::Traffic, SegmentFilter { traffic: true, ..Default::default() }),
            false => (NavKind::Direction, LaneType::Cop, SegmentFilter { cop: true, ..Default::default() }),
        };
        let mut nav = RoadNav::new(kind, lane, filter);
        nav.half_width = self.half.0;
        if !nav.init_at_point(net, index, self.physics_position(), self.physics_forward(), false) {
            return;
        }
        if searching && !nav.can_traffic_spawn(net, rng) {
            return;
        }
        nav.enable_trail(net);
        self.nav = nav;
        self.searching = searching;
        self.cruise = cruise;
        self.arrived = false;
        self.waiting = false;
        self.stop_memory = None;
        self.goal.set(GoalDrive::default());
        self.target = self.nav.position;
        let speed = self.speed().abs();
        if let Role::Cop(cop) = &mut self.role {
            cop.resume_chase(speed);
        }
    }

    /// The original pursuit goal selects its lane-following traffic action after tracked sight expires.
    pub(super) fn think_search<R: RandomSource>(&mut self, ctx: &mut Ctx<'_, R>, state: &traffic::CarState) {
        let think = traffic::think(
            ctx.net,
            &mut self.nav,
            state,
            traffic::others(ctx.bodies, ctx.me, ctx.player),
            self.blocked,
            ctx.rng,
            &self.cruise,
        );
        self.target = think.target;
        self.speed = think.speed;
        self.blocked = think.blocked;
    }
}

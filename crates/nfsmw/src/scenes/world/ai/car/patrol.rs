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
        let mut nav =
            RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter { cop: true, ..Default::default() });
        nav.half_width = self.half.0;
        if !nav.init_at_point(net, index, self.physics_position(), self.physics_forward(), false) {
            return false;
        }
        nav.preserve_offset_at(net, self.physics_position());
        nav.enable_trail(net);
        self.nav = nav;
        self.role = Role::Cop(Box::new(cop::CopState::new(car, player, self.speed().abs())));
        self.sight.without_target();
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
        (self.is_cop() || self.roadblock) && !self.is_wrecked() && !self.arrested && !self.fleeing
    }

    /// The cop was sent away from the chase.
    pub fn is_fleeing(&self) -> bool {
        self.fleeing
    }

    /// Whether the cop has been within the engagement radius of the player in this chase.
    pub fn was_engaged(&self) -> bool {
        self.engaged
    }

    /// Notes that the cop is within the engagement radius.
    pub fn note_engaged(&mut self) {
        self.engaged = true;
    }

    /// Sends the cop away (`AIVehiclePursuit::StartFlee`): its lights go off and it drives off along the traffic lanes at
    /// the search speed, like the cops that lost the player, until the patrol pass recycles it. Support cars and roadblock
    /// cars are not sent this way. Returns whether the cop now flees; a cop with no lane to follow keeps chasing.
    pub(in crate::scenes::world::ai) fn start_flee(
        &mut self,
        cruise: Cruise,
        net: &RoadNetwork,
        index: &SegmentIndex,
    ) -> bool {
        if !self.is_cop() || self.roadblock || self.support.is_some() || self.arrested {
            return false;
        }
        if self.fleeing {
            return true;
        }
        if !self.searching {
            self.search_mode(true, cruise, net, index);
        }
        if !self.searching {
            return false;
        }
        self.fleeing = true;
        true
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
    pub(in crate::scenes::world::ai) fn test_target_sight(&mut self, hidden: bool, visible: bool) -> bool {
        if hidden && self.perp_hidden_from_me {
            self.sees_player = false;
            return false;
        }
        self.perp_hidden_from_me = hidden && !visible;
        self.sees_player = visible;
        if visible {
            self.saw_target();
        }
        visible
    }

    /// Force a fixture's observation without advancing its physical vehicle task.
    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn report_sight(&mut self, hidden: bool, visible: bool) -> bool {
        let seen = self.test_target_sight(hidden, visible);
        if !seen {
            self.sight.without_target();
        }
        seen
    }

    /// Whether this cop saw the player at the last sight test (the per-car answer behind the flee rule's "can see you").
    pub fn sees_player(&self) -> bool {
        match self.is_cop() {
            true => self.sight_age() <= 0.0,
            false => self.sees_player,
        }
    }

    /// How this car stands to a cop about to be put on the traffic lanes (`CheckSpawnPosition`'s lane test): a car that
    /// follows the traffic lanes gives its lane and the end it heads for, a wreck of one blocks its lane, any other car
    /// is only held off by the general distance.
    pub fn spacing_neighbour(&self) -> blackbox_pursuit::spawn_rules::Neighbour {
        use blackbox_pursuit::spawn_rules::Neighbour;
        if self.nav.kind != NavKind::Traffic {
            return Neighbour::Other;
        }
        if !self.is_wrecked()
            && let Some((from_lane, to_lane)) = self.avoid.changing_lanes()
        {
            return Neighbour::ChangingTraffic { from_lane, to_lane, node: self.nav.node_ind };
        }
        match self.is_wrecked() {
            true => Neighbour::Wreck,
            false => Neighbour::Traffic { lane: self.nav.lane, node: self.nav.node_ind },
        }
    }

    /// Switch actions without moving or replacing the physical vehicle.
    pub(in crate::scenes::world::ai) fn search_mode(
        &mut self,
        searching: bool,
        cruise: Cruise,
        net: &RoadNetwork,
        index: &SegmentIndex,
    ) {
        if !self.is_cop()
            || self.roadblock
            || self.support.is_some()
            || self.searching == searching
            || self.arrested
            || self.fleeing
        {
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

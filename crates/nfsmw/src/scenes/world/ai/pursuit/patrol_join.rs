//! Patrol cars joining the chase (spec `ai-pursuit-cops.md` section 2 and `ai-pursuit.md` section 4.2). A patrol that
//! sees the player asks for a pursuit; once a manager run, the first such patrol (in car order) joins the chase of its
//! target, or starts it when there is none. A chase therefore begins with the one patrol that saw the player, and the
//! other patrols that see it come in one at a time, each restarting the reinforcement timer, as do patrols that spot the
//! player later in the chase. A patrol that does not see the player stays a patrol, however near it is.

use super::*;

impl TrafficWorld {
    /// The patrol car that asks for a pursuit now: the first live patrol, in car order, that saw the player at its last
    /// sight test. A roadblock car or a cop sent away never asks.
    fn requesting_patrol(&self) -> Option<usize> {
        self.cars.iter().position(|car| {
            car.is_patrol() && !car.is_roadblock() && !car.is_fleeing() && !car.is_wrecked() && car.sees_player()
        })
    }

    /// One manager run's patrol join: the requesting patrol, if any, becomes a chase cop in place and counts among the
    /// cops involved. Returns whether one joined.
    pub(super) fn join_requesting_patrol(&mut self, cooldown: bool) -> bool {
        let Some(i) = self.requesting_patrol() else { return false };
        let Some(pursuit) = self.pursuit.as_ref() else { return false };
        let player = pursuit.player;
        let name = self.cars[i].name.clone();
        let Some(model) = self.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case(&name)) else {
            return false;
        };
        if !self.cars[i].join_pursuit(model.car, player, &self.network, &self.index) {
            return false;
        }
        if let Some(pursuit) = self.pursuit.as_mut() {
            pursuit.joined(cooldown);
        }
        self.police.ground_deployed(&name);
        true
    }
}

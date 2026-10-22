//! Where the streaming follows the view: the car on the road, or the free camera. In co-op every player's car is a
//! centre of its own (`docs/specs/coop-career.md` section 10).

use super::drive::Drive;
use super::{View, WorldScene};

/// The map position (x, y) streaming follows for one car: its spawn request while it waits for a road, else the car.
fn car_focus(drive: &Drive) -> [f32; 2] {
    match drive.waiting_for_road() {
        Some(request) => request.near,
        None => {
            let p = drive.position();
            [p.x, p.y]
        }
    }
}

impl WorldScene {
    /// Map position (x, y) the streaming follows for the lead (or the free camera).
    pub(super) fn focus(&self) -> [f32; 2] {
        match (self.players.lead(), self.view) {
            (Some(drive), View::Chase) => car_focus(drive),
            _ => [self.camera.position.x, self.camera.position.y],
        }
    }

    /// Every streaming centre, the lead's first: then each co-op guest's car, and the second racer of a split-screen
    /// race. One centre in single player.
    pub(super) fn foci(&self) -> Vec<[f32; 2]> {
        self.foci_with_speed().map(|(focus, _)| focus).collect()
    }

    /// The streaming centres with the speed of the car at each (0 for the free camera), for the load margin.
    pub(super) fn foci_with_speed(&self) -> impl Iterator<Item = ([f32; 2], f32)> + '_ {
        let lead = self.players.lead().map_or(0.0, |drive| drive.speed().abs());
        let guests = match self.view {
            View::Chase => Some(self.players.guests().map(|g| (car_focus(&g.drive), g.drive.speed().abs()))),
            View::Fly => None,
        };
        let racer = self.second_player_position().map(|p| ([p.x, p.y], 0.0));
        std::iter::once((self.focus(), lead)).chain(guests.into_iter().flatten()).chain(racer)
    }
}

//! Where the streaming follows the view: the car on the road, or the free camera.

use super::{View, WorldScene};

impl WorldScene {
    /// Map position (x, y) the streaming follows.
    pub(super) fn focus(&self) -> [f32; 2] {
        match (&self.drive, self.view) {
            (Some(drive), View::Chase) => match drive.waiting_for_road() {
                Some(request) => request.near,
                None => {
                    let p = drive.position();
                    [p.x, p.y]
                }
            },
            _ => [self.camera.position.x, self.camera.position.y],
        }
    }
}

//! The first-car pick of the car lot (`JumpToCarLot`, `docs/specs/story-flow.md` section 2.3): the owned car under the cursor
//! becomes the current car, and the drive to the first safe house starts in it (`flow_prologue.rs`).

use super::car_lot::{Key, Tab, items};
use crate::career::CareerHandle;

/// Makes the owned car at `cursor` the current car. `false` when that row is not an owned car (a lot row, or no row).
pub(super) fn pick_first_car(cursor: usize) -> bool {
    CareerHandle::shared()
        .with(|career| {
            let Key::Owned(handle) = items(career, Tab::Owned).into_iter().nth(cursor)?.key else {
                return None;
            };
            career.settings.car_handle = handle;
            Some(())
        })
        .flatten()
        .is_some()
}

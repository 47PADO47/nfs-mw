//! The car select's authored loading LEDs (`Car_Select.fng`, group `0x6A0831F6`) run while the backdrop car loads, as in
//! the recorded original. Packages without that row have no route for the messages and ignore them.

use blackbox_feng::PackageId;

use super::Screens;
use crate::frontend::ids::{CAR_LOADING_HIDE, CAR_LOADING_SHOW};

impl Screens {
    /// Tells the top screen whether the backdrop car is still loading, once per change of screen or state.
    pub fn set_car_loading(&mut self, loading: bool) {
        let now = self.stack.last().map(|screen| (screen.id, loading));
        let previous = std::mem::replace(&mut self.car_loading, now);
        let Some((package, message)) = signal(previous, now) else { return };
        self.runtime.post_to_package(package, message);
    }
}

/// The package and message a change needs; a new top screen hears the current state, a kept one only changes.
fn signal(previous: Option<(PackageId, bool)>, now: Option<(PackageId, bool)>) -> Option<(PackageId, u32)> {
    if previous == now {
        return None;
    }
    let (package, loading) = now?;
    Some((package, if loading { CAR_LOADING_SHOW } else { CAR_LOADING_HIDE }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r2_loading_leds_follow_each_change_and_a_new_top_screen() {
        let (a, b) = (PackageId(1), PackageId(2));
        assert_eq!(signal(None, Some((a, true))), Some((a, CAR_LOADING_SHOW)));
        assert_eq!(signal(Some((a, true)), Some((a, true))), None, "no repeat while nothing changes");
        assert_eq!(signal(Some((a, true)), Some((a, false))), Some((a, CAR_LOADING_HIDE)));
        assert_eq!(signal(Some((a, false)), Some((b, true))), Some((b, CAR_LOADING_SHOW)));
        assert_eq!(signal(Some((b, true)), None), None);
    }
}

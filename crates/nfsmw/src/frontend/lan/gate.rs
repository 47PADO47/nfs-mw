//! The front end's frame meets the LAN menu: the car select the menu asked for opens, and while the menu is on screen the
//! FEng screens take no input (the menu took this frame's keys and script step in `system.rs`).

use crate::app::Host;
use crate::frontend::flow::Frontend;
use crate::frontend::ids::screen;
use crate::frontend::logic::Env;

use super::link;

/// Called by the front end's frame once the screens are ready. `true` when the LAN menu has the input: the frame stops.
pub(in crate::frontend) fn gate(fe: &mut Frontend, host: &mut Host, env: &mut Env) -> bool {
    if let Some(args) = link::take_car_select() {
        fe.open_menu(screen::CAR_SELECT, args, host, env);
    }
    if !fe.lan_input {
        return false;
    }
    // What the rest of the frame would have done for the host and the menus' unsaved settings.
    host.flow_driven = true;
    host.cancel_handled = true;
    host.hold_capture = fe.script.as_ref().is_some_and(|script| !script.is_over());
    fe.changed = std::mem::take(env.changed);
    true
}

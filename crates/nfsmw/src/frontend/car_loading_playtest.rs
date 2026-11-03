//! R2: the car select's authored loading LEDs fade in while the backdrop car loads and out once it is shown.

use super::*;
use crate::frontend::logic::{Cx, ScreenLogic};
use crate::input::{Bindings, InputDevice};

/// The installed package's own logic is not needed: only its authored LED row is read.
struct Bare;
impl ScreenLogic for Bare {
    fn message(&mut self, _: &mut Cx, _: u32) {}
}

/// The LED row group of `Car_Select.fng`.
const LED_ROW: u32 = 0x6A08_31F6;

fn row_alpha(h: &Harness) -> u8 {
    let pages = h.screens.presented(&Bindings::default(), InputDevice::Xbox);
    let tree = &pages.last().expect("car select").tree;
    let row = tree.nodes.iter().find(|node| node.name_hash == LED_ROW).expect("authored loading row");
    if row.visible { row.world_colour[3] } else { 0 }
}

#[test]
#[ignore = "requires the installed car select package; set NFSMW_GAME_DIR"]
fn r2_car_select_loading_leds_fade_in_while_loading_and_out_when_shown() {
    let mut h = Harness::open(screen::MAIN_MENU, Args::default()).expect("installed menus");
    let mut env = Env { settings: &mut h.settings, changed: &mut h.changed, caps: test_caps::native(), mods: None };
    h.screens.open_with("Car_Select.fng", Box::new(Bare), false, &mut env);
    h.wait(1.0);
    assert_eq!(row_alpha(&h), 0, "no loading row while nothing loads");
    h.screens.set_car_loading(true);
    h.wait(0.5);
    assert!(row_alpha(&h) > 128, "the row fades in while the car loads: {}", row_alpha(&h));
    h.screens.set_car_loading(false);
    h.wait(1.0);
    assert_eq!(row_alpha(&h), 0, "the row fades out once the car is shown");
}

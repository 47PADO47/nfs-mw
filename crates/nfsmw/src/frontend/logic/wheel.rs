//! The mouse wheel on a screen that is a list.

use blackbox_feng::ids::{PAD_DOWN, PAD_UP};

use super::{Cx, ScreenLogic};

/// Each notch is the Up or Down press, so the wheel moves the selection as the pad does and a list needs no wheel code of
/// its own. Positive `steps` are up.
pub fn wheel_as_rows<L: ScreenLogic + ?Sized>(logic: &mut L, cx: &mut Cx, steps: i32) {
    let press = if steps > 0 { PAD_UP } else { PAD_DOWN };
    for _ in 0..steps.unsigned_abs() {
        logic.message(cx, press);
    }
}

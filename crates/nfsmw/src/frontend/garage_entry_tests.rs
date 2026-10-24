use super::*;

#[test]
fn main_menu_my_cars_dispatches_the_normal_garage() {
    let Some(mut h) = Harness::open(screen::MAIN_MENU, Args::default()) else { return };
    h.wait(1.0);
    for _ in 0..3 {
        h.press(pad::RIGHT);
    }
    h.press(pad::ACCEPT);
    h.wait(1.5);
    assert!(h.said(&Command::OpenGarage), "{:?}", h.commands);
}

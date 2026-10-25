//! The cop icon's colour cycle, with the numbers of `docs/specs/hud-cop-blips.md`.

use super::cop_blip::{ICON_COUNT, colour, icon_name, pursuit_colour, step_at};

#[test]
fn the_icons_are_named_by_their_index() {
    assert_eq!(ICON_COUNT, 8);
    assert_eq!(icon_name(0), "MMICON_COPCAR_0");
    assert_eq!(icon_name(7), "MMICON_COPCAR_7");
}

#[test]
fn the_light_bar_cycles_blue_grey_red_grey() {
    let blue = [0, 0, 255, 255];
    let grey = [204, 204, 204, 255];
    let red = [160, 0, 0, 255];
    let cycle: Vec<[u8; 4]> = (1..=8).map(pursuit_colour).collect();
    assert_eq!(cycle, [blue, blue, grey, grey, red, red, grey, grey]);
}

#[test]
fn the_step_follows_the_frame() {
    assert_eq!(step_at(0), 1);
    assert_eq!(step_at(7), 8);
    assert_eq!(step_at(8), 1);
    assert_eq!(step_at(9), 2);
}

#[test]
fn a_car_that_is_not_pursuing_is_white_whatever_the_step() {
    for step in 1..=8 {
        assert_eq!(colour(false, step), [255, 255, 255, 255]);
    }
}

#[test]
fn a_pursuing_car_follows_the_light_bar() {
    assert_eq!(colour(true, 1), [0, 0, 255, 255]);
    assert_eq!(colour(true, 3), [204, 204, 204, 255]);
    assert_eq!(colour(true, 5), [160, 0, 0, 255]);
}

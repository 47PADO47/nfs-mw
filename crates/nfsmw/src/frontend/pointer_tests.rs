use game_install::GameDir;

use super::*;
use crate::ui::UiAssets;

#[test]
#[ignore = "requires an unmodified base PC install; set NFSMW_GAME_DIR"]
fn game_pointer_is_a_16_pixel_arrow_with_its_tip_inside() {
    let path = std::env::var("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to an unmodified base PC install");
    let dir = GameDir::open(path).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    let image = pointer_image(&assets).expect("the install has PC_CURSOR");
    assert_eq!((image.width(), image.height()), (16, 16));
    assert!(u32::from(HOTSPOT.0) < image.width() && u32::from(HOTSPOT.1) < image.height());
    let shown = image.data.as_ref().expect("the pixels are kept for the window");
    assert!(shown.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
}

#[test]
fn the_hotspot_follows_an_hd_cursor_only_when_the_option_is_on() {
    assert_eq!(hotspot((16, 16), true), HOTSPOT);
    assert_eq!(hotspot((16, 16), false), HOTSPOT);
    assert_eq!(hotspot((32, 32), true), (4, 4));
    assert_eq!(hotspot((32, 32), false), HOTSPOT);
    assert_eq!(hotspot((64, 32), true), (8, 4), "each axis scales by its own ratio");
}

#[test]
fn the_hotspot_stays_inside_a_small_texture() {
    let (x, y) = hotspot((4, 4), true);
    assert!(x <= 3 && y <= 3, "{x}, {y}");
    assert_eq!(hotspot((1, 1), true), (0, 0));
}

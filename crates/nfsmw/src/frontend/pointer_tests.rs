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

//! Cart rows retain their authored horizontal spacing; rendered glyph centers govern vertical alignment.
use super::*;
use crate::ui::{Catalog, SCREEN_FILES};
use game_install::GameDir;

#[test]
#[ignore = "requires installed original garage packages"]
fn installed_cart_check_tracks_the_item_label() {
    let mut h = Harness::open(Page::Main, model());
    h.ui.preview(Row::Part(Part::Body), 1);
    h.ui.add_to_cart();
    h.flush();
    h.ui.show_cart();
    h.flush();
    h.run(0, 120);
    let dir = GameDir::open(std::path::PathBuf::from(std::env::var_os("NFSMW_GAME_DIR").unwrap())).unwrap();
    let catalog = Catalog::load(&dir, &SCREEN_FILES);
    let package = catalog.find(h.top()).unwrap();
    let find = |name: &str| package.objects.iter().find(|o| o.name_hash == blackbox_feng::fe_hash_upper(name)).unwrap();
    let name = find("PART_NAME_1");
    let check = find("CHECK_ICON_1");
    assert_eq!(name.parent, check.parent, "authored peers share coordinates");
    let expected = check.data.position() - name.data.position();
    let tree = h.screens.trees().pop().unwrap();
    let node = |hash| tree.nodes.iter().find(|n| n.name_hash == hash).unwrap();
    let label = node(name.name_hash);
    let icon = node(check.name_hash);
    assert!(label.visible && icon.visible);
    let actual = icon.local_position - label.local_position;
    assert!(
        (actual.x - expected.x).abs() < 0.01,
        "check and label retain horizontal package spacing: {actual:?} vs {expected:?}"
    );
}

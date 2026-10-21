//! The screen's names against the hashes the package and the decompiled code use.

use blackbox_feng::fe_hash_upper;

use super::*;

#[test]
fn the_objects_the_screen_writes_are_the_packages_by_name() {
    assert_eq!(MAP, fe_hash_upper("TRACK_MAP"));
    assert_eq!(NUMBER, fe_hash_upper("NUMBER_GROUP"));
    assert_eq!(NUMBER_OF, fe_hash_upper("NUMBER_OF_GROUP"));
    assert_eq!(EVENT_ICON, fe_hash_upper("EVENT_ICON"));
    assert_eq!(DESCRIPTION, fe_hash_upper("EVENT_DESCRIPTION"));
    assert_eq!(fe_hash_upper("EVENT_ICON_1"), 0x09C9_A0F9);
    assert_eq!(fe_hash_upper("CHECK_1"), 0xE636_228D);
    assert_eq!(fe_hash_upper("CHECK_10"), 0xACFA_745D);
    assert_eq!(fe_hash_upper("SHOW_HELP"), SHOW_HELP);
}

#[test]
fn the_scripts_are_the_packages() {
    assert_eq!(fe_hash_upper("HIGHLIGHT"), HIGHLIGHT);
    assert_eq!(fe_hash_upper("UNHIGHLIGHT"), UNHIGHLIGHT);
    assert_eq!(fe_hash_upper("INIT"), INIT);
    assert_eq!(fe_hash_upper("APPEAR"), APPEAR);
}

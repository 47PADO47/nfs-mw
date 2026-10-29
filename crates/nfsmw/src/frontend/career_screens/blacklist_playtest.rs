//! Installed-package reproduction for UI-032: a bio must bind its own rival media.

use blackbox_feng::{NodeKind, fe_hash_upper};

use super::*;
use crate::frontend::career_screens::rep_sheets::playtest::Harness;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn career_playtest_bio_uses_the_requested_rival_background_and_graffiti() {
    let mut h = Harness::installed();
    for bin in [1, 7, 15] {
        h.open(screen::SAFE_HOUSE_RIVAL_BIO, Box::new(BioLogic::new(Args { rival: bin, ..Args::default() })));
        for (hash, suffix) in [(0x2CBE_1DD0, "_BG"), (0xF5A2_A087, "_GRAF")] {
            let node = h.node(hash);
            assert!(node.visible, "rival {bin} {suffix} remains hidden");
            let texture = fe_hash_upper(&format!("RIVAL_{bin:02}{suffix}"));
            assert!(
                matches!(node.kind, NodeKind::Image { texture: key, .. } if key == texture),
                "rival {bin} {suffix}: {:?}",
                node.kind
            );
        }
        assert!(
            h.tree().nodes.iter().any(|n| n.visible && n.text.as_ref().is_some_and(|text| text.len() > 100)),
            "bio body"
        );
    }
}

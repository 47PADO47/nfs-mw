//! Native hints replaced by the host footer, including their animated boxes.

use super::LEGACY_HINTS;
use crate::{
    frontend::{ids, prompt_layout},
    i18n,
};
use blackbox_feng::{NodeKind, UiTree};

pub(super) fn hide(tree: &mut UiTree, splash: bool, brief: bool, settings: bool, jukebox: bool) {
    // 0xD646_3100 is the Game Stats (and Randomize) button group.
    // 0xB8A7_C6CE is the profile list's own Accept; 0x28FD_842D the main menu's Game Stats card header.
    let names = [
        0x6A21_8478,
        0x812A_09D4,
        0x7379_349B,
        0x5E4D_1BDC,
        0xD646_3100,
        0xB8A7_C6CE,
        0x28FD_842D,
        ids::QUIT_BUTTON,
        // The overview's legacy Back triangle is a direct footer child, outside the Back text group.
        0x1AC9_4CA9,
    ];
    // The draw tree does not keep the labels its strings came from, so a package's own hint is known by the text its
    // label has in the current language.
    let words: Vec<String> = LEGACY_HINTS.iter().filter_map(|label| i18n::game(*label)).collect();
    let mut hidden = vec![false; tree.nodes.len()];
    for (i, n) in tree.nodes.iter().enumerate() {
        let hint_text = n.text.as_deref().is_some_and(|t| words.iter().any(|word| word == t.trim()));
        // The brief's Randomize root has the generic name "Group", shared by unrelated screens.
        let brief_hint = brief && [prompt_layout::BRIEF_FOOTER, prompt_layout::BRIEF_RANDOMIZE].contains(&n.name_hash);
        // PC key art can live outside caption groups and be shown again by the exit scripts.
        let settings_key = settings
            && (matches!(n.kind, NodeKind::Image { texture: 0xd7b6_edb7 | 0x1bb3_c514, .. })
                || (n.name_hash == 0xe795_717d
                    && n.parent
                        .and_then(|p| tree.nodes.get(p))
                        .is_some_and(|p| [0x84bc_f2b5, 0xb3a6_9e89].contains(&p.name_hash))));
        // These are the jukebox's old PC hint roots and controller hint groups, not its footer bar.
        let jukebox_hint =
            jukebox && [0x01bc_fc29, 0x73dc_b662, 0x7f02_b4cd, 0xff88_751e, 0x3fd9_6ce9].contains(&n.name_hash);
        hidden[i] = jukebox_hint
            || hint_text
            || names.contains(&n.name_hash)
            || brief_hint
            || settings_key
            || (splash && n.name_hash == 0xC4DF_3FF2);
    }
    // The PC hint owns a caption, key and striped box. Hide that complete hint, not just its glyph.
    if settings {
        for node in &tree.nodes {
            if !matches!(node.kind, NodeKind::Image { texture: 0xd7b6_edb7 | 0x1bb3_c514, .. }) {
                continue;
            }
            let Some(parent) = node.parent.filter(|p| *p < hidden.len()) else { continue };
            if [0x84bc_f2b5, 0xb3a6_9e89].contains(&tree.nodes[parent].name_hash) {
                continue;
            }
            hidden[parent] = true;
        }
    }
    // Ancestors precede descendants in the actual packages; the parent chain also handles other orderings.
    for i in 0..tree.nodes.len() {
        let mut parent = tree.nodes[i].parent;
        let mut budget = tree.nodes.len();
        while let Some(p) = parent.filter(|p| *p < tree.nodes.len()) {
            if hidden[p] {
                hidden[i] = true;
                break;
            }
            if budget == 0 {
                break;
            }
            budget -= 1;
            parent = tree.nodes[p].parent;
        }
    }
    // The composed menu slot copies whole trees (`ui/theme/original/menu.rs`) and reads `visible`, not `draw_order`,
    // so a hidden node is marked invisible as well as dropped from the draw order.
    for (node, hidden) in tree.nodes.iter_mut().zip(&hidden) {
        node.visible &= !hidden;
    }
    tree.draw_order.retain(|i| !hidden[*i]);
}

/// Hide old footer content, retaining its animated root and background strip.
pub(super) fn hide_children(tree: &mut UiTree, hash: u32) {
    let Some(root) = tree.nodes.iter().find(|n| n.name_hash == hash).map(|n| n.index) else { return };
    let mut hidden = vec![false; tree.nodes.len()];
    for (i, hide) in hidden.iter_mut().enumerate() {
        if i == root {
            continue;
        }
        let mut at = tree.nodes[i].parent;
        for _ in 0..tree.nodes.len() {
            let Some(p) = at.filter(|p| *p < tree.nodes.len()) else { break };
            if p == root {
                let background = tree.nodes[i].parent == Some(root)
                    && matches!(tree.nodes[i].kind, NodeKind::Image { texture: 0xc6af_dd7e | 0x397a_3a31, .. });
                *hide = !background;
                break;
            }
            at = tree.nodes[p].parent;
        }
    }
    for (n, hidden) in tree.nodes.iter_mut().zip(&hidden) {
        n.visible &= !hidden;
    }
    tree.draw_order.retain(|i| !hidden[*i]);
}

/// Suppress complete obsolete action cards outside a shared footer.
pub(super) fn hide_roots(tree: &mut UiTree, hashes: &[u32]) {
    let roots: Vec<_> = tree.nodes.iter().filter(|n| hashes.contains(&n.name_hash)).map(|n| n.index).collect();
    let mut hidden = vec![false; tree.nodes.len()];
    for (i, hide) in hidden.iter_mut().enumerate() {
        let mut at = Some(i);
        for _ in 0..tree.nodes.len() {
            let Some(p) = at.filter(|p| *p < tree.nodes.len()) else { break };
            if roots.contains(&p) {
                *hide = true;
                break;
            }
            at = tree.nodes[p].parent;
        }
    }
    for (n, hide) in tree.nodes.iter_mut().zip(&hidden) {
        n.visible &= !hide;
    }
    tree.draw_order.retain(|i| !hidden[*i]);
}

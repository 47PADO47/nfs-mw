//! The cached node ownership gives the same answer as the walk up the parents it replaces, and is worked out again
//! for another shape of tree.

use std::collections::HashMap;
use std::sync::Arc;

use blackbox_ui::{UiNode, UiTree};

use super::structure::Structure;
use super::subtree::{Scope, keep_flags};
use super::{FengSource, FrameCell};

fn node(index: usize, name_hash: u32, parent: Option<usize>, visible: bool) -> UiNode {
    UiNode { index, name_hash, parent, visible, ..UiNode::default() }
}

const ROOT: u32 = 10;
const EXCLUDED: u32 = 30;

fn scope() -> Scope {
    Scope::Under { roots: Arc::from([ROOT]), excluded: Arc::from([EXCLUDED]) }
}

/// A visible root with a plain child, an excluded child and its own child, a stray node, two nodes that are each
/// other's parent, and a hidden root with a child.
fn tree() -> UiTree {
    UiTree {
        nodes: vec![
            node(0, ROOT, None, true),
            node(1, 20, Some(0), true),
            node(2, EXCLUDED, Some(1), true),
            node(3, 40, Some(2), true),
            node(4, 50, None, true),
            node(5, 60, Some(6), true),
            node(6, 70, Some(5), true),
            node(7, ROOT, None, false),
            node(8, 80, Some(7), false),
            node(9, 90, Some(99), true),
        ],
        draw_order: Vec::new(),
    }
}

fn walked(tree: &UiTree, scope: &Scope) -> Vec<bool> {
    let by_index: HashMap<usize, usize> = tree.nodes.iter().enumerate().map(|(pos, n)| (n.index, pos)).collect();
    keep_flags(tree, &by_index, scope)
}

fn cached(tree: &UiTree, scope: &Scope) -> Vec<bool> {
    let mut structure = Structure::new(tree);
    let slot = structure.owner_slot(tree, scope);
    (0..tree.nodes.len()).map(|pos| structure.keeps(tree, slot, pos)).collect()
}

#[test]
fn the_cached_owners_match_the_walk_up_the_parents() {
    let tree = tree();
    let kept = cached(&tree, &scope());
    assert_eq!(kept, walked(&tree, &scope()));
    // The root and its plain child are taken; the excluded subtree, the stray nodes and the hidden root are not.
    assert_eq!(&kept[..4], &[true, true, false, false]);
    assert!(kept[4..].iter().all(|k| !k));
}

#[test]
fn a_child_listed_before_its_parent_finds_it_by_index() {
    let mut tree = tree();
    tree.nodes.reverse();
    assert_eq!(cached(&tree, &scope()), walked(&tree, &scope()));
    assert!(cached(&tree, &scope()).iter().any(|k| *k));
}

#[test]
fn a_name_in_both_lists_counts_as_excluded() {
    let tree = tree();
    let both = Scope::Under { roots: Arc::from([ROOT, EXCLUDED]), excluded: Arc::from([EXCLUDED]) };
    assert_eq!(cached(&tree, &both), walked(&tree, &both));
}

#[test]
fn a_scope_is_worked_out_once_per_tree() {
    let (frame, tree) = (FrameCell::new(), tree());
    frame.set(FengSource::Hud, tree.clone());
    let owners = |frame: &FrameCell, tree: &UiTree| {
        frame.with_structure(tree, |structure| {
            structure.owner_slot(tree, &scope());
            structure.owner_slot(tree, &scope());
            structure.owners.len()
        })
    };
    assert_eq!(owners(&frame, &tree), 1, "the second ask finds the first answer");
    assert_eq!(owners(&frame, &tree), 1, "so does a later frame");
    // A tree of another shape starts over: it has nothing worked out yet when it is first read.
    let mut other = tree.clone();
    other.nodes.pop();
    frame.set(FengSource::Hud, other.clone());
    let remembered = frame.with_structure(&other, |structure| structure.owners.len());
    assert_eq!(remembered, 0);
}

#[test]
fn the_same_tree_set_again_keeps_what_was_worked_out() {
    let (frame, tree) = (FrameCell::new(), tree());
    frame.set(FengSource::Hud, tree.clone());
    frame.with_structure(&tree, |structure| structure.owner_slot(&tree, &scope()));
    frame.fill(FengSource::Hud, |t| *t = tree.clone());
    let remembered = frame.with_structure(&tree, |structure| structure.owners.len());
    assert_eq!(remembered, 1, "the shape did not change, so the owners are still right");
}

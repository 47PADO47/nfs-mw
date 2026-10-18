//! The Original HUD adapter: the FEng subtree under a root of one package's tree, copied into the caller's draw list.
//!
//! A slot's objects are a root group and its children (`docs/specs/ui-themes.md` section 8, point 2). The adapter reads
//! the frame's tree for its package from the shared cell ([`FrameCell`]) and emits the nodes that belong to its scope.
//! It does not own the tree, runs no script and changes no FEng state.
//!
//! Rules the emitted nodes follow:
//!
//! - **Membership.** A node belongs to a root when walking up from the node (itself included) meets that root's name
//!   before it meets an excluded name. An excluded name is a subtree another element owns (the gear inside the
//!   tachometer, the blips inside the minimap), so no node is emitted by two adapters.
//! - **Hidden or absent root.** A root that is not in the tree, or is not visible, takes its whole subtree with it:
//!   nothing is emitted. Visibility is inherited from the parent in the producer, so a hidden root has no visible
//!   child either; skipping the subtree only saves the work.
//! - **Order.** Nodes are emitted in the tree's own order (the package's object order, parents first). The composed
//!   draw order is then `DrawList::into_tree`'s, a stable sort by `z`, which is what FEng's `draw_order` is.
//! - **Links.** A node's `parent` is remapped into the draw list's index space. The draw list may already hold nodes
//!   from other adapters, so the remap starts at `out.len()`. A root's parent lies outside the subtree and becomes
//!   `None`. `index` is kept: it stays the package's object index.
//! - **Layout.** `layout` does nothing. FEng already placed every object in world space (`UiNode::world`), so the
//!   nodes are copied as they are, including the world transform.

use std::collections::HashMap;

use blackbox_ui::{Component, DrawList, LayoutCx, Rect, UiTree};

use super::frame::{FengSource, FrameCell};

/// Which nodes of a tree an adapter emits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Scope {
    /// The nodes under one of `roots` (name hashes), except those under an `excluded` name.
    Under { roots: Vec<u32>, excluded: Vec<u32> },
    /// Every node of the package.
    Package,
}

/// An adapter that emits one FEng subtree of one package each time it is built, read from the frame cell.
///
/// Built by `hud_slots::adapter` for each HUD slot and by `OriginalTheme::element`; it is a [`Component`], so the
/// presenter and the composer treat it like any other.
pub struct FengSubtree {
    frame: FrameCell,
    source: FengSource,
    scope: Scope,
}

impl FengSubtree {
    /// The subtree under each of `roots`, in `source`'s tree, leaving out the subtrees under `excluded`.
    ///
    /// If a name is in both lists, the excluded one wins: a node under it belongs to another element.
    pub fn under(frame: &FrameCell, source: FengSource, roots: Vec<u32>, excluded: Vec<u32>) -> Self {
        Self { frame: frame.clone(), source, scope: Scope::Under { roots, excluded } }
    }

    /// Every node of `source`'s package: the chyron and the busted overlay, which have no element root of their own.
    pub fn package(frame: &FrameCell, source: FengSource) -> Self {
        Self { frame: frame.clone(), source, scope: Scope::Package }
    }
}

impl Component for FengSubtree {
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        let Some(tree) = self.frame.get(self.source) else { return };
        emit(&tree, &self.scope, out);
    }
}

/// Appends the nodes of `tree` that `scope` takes to `out`, in the tree's order, with `parent` links remapped to
/// `out`'s positions. Pure: the same tree and scope always give the same nodes.
pub(super) fn emit(tree: &UiTree, scope: &Scope, out: &mut DrawList) {
    let by_index: HashMap<usize, usize> = tree.nodes.iter().enumerate().map(|(pos, node)| (node.index, pos)).collect();
    let keep = keep_flags(tree, &by_index, scope);
    // The position each kept node gets in `out`. Kept nodes keep their relative order.
    let mut next = out.len();
    let placed: Vec<Option<usize>> = keep
        .iter()
        .map(|kept| {
            kept.then(|| {
                next += 1;
                next - 1
            })
        })
        .collect();
    for (pos, node) in tree.nodes.iter().enumerate() {
        if !keep[pos] {
            continue;
        }
        let mut copy = node.clone();
        copy.parent = node.parent.and_then(|parent| by_index.get(&parent)).and_then(|at| placed[*at]);
        out.push(copy);
    }
}

/// Which positions of `tree` `scope` takes.
fn keep_flags(tree: &UiTree, by_index: &HashMap<usize, usize>, scope: &Scope) -> Vec<bool> {
    match scope {
        Scope::Package => vec![true; tree.nodes.len()],
        Scope::Under { roots, excluded } => (0..tree.nodes.len())
            .map(|pos| {
                let owner = owner(tree, by_index, roots, excluded, pos);
                owner.is_some_and(|root| tree.nodes[root].visible)
            })
            .collect(),
    }
}

/// The position of the root that the node at `pos` belongs to, or `None` when it belongs to no root or to an
/// excluded subtree. Walks up from `pos`: the first node whose name is excluded stops the walk, and the first node
/// whose name is a root is the owner.
fn owner(
    tree: &UiTree,
    by_index: &HashMap<usize, usize>,
    roots: &[u32],
    excluded: &[u32],
    pos: usize,
) -> Option<usize> {
    let mut at = Some(pos);
    // A parent chain is at most as long as the tree, so this bound also stops a cycle in corrupt data.
    for _ in 0..tree.nodes.len() {
        let current = at?;
        let node = &tree.nodes[current];
        if excluded.contains(&node.name_hash) {
            return None;
        }
        if roots.contains(&node.name_hash) {
            return Some(current);
        }
        at = node.parent.and_then(|parent| by_index.get(&parent).copied());
    }
    None
}

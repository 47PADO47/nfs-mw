//! What a HUD package's tree says about itself and never changes: which node is whose parent, and which root owns each
//! node for a scope.
//!
//! A package's objects, names and parent links are fixed when it loads; only visibility and placement move. So the
//! walk up the parents that decides the owner of a node (`FengSubtree`'s membership rule) is done once per scope and
//! package, here, and a frame only reads the answer and checks the owning root's visibility. The cache is checked
//! against the tree it is asked about (index, name and parent of every node), so a different package, or a tree of
//! another shape, builds it again.

use std::collections::HashMap;

use blackbox_ui::UiTree;

use super::subtree::Scope;

/// "No position": a node with no owner, or no parent in the tree.
pub(super) const NONE: u32 = u32::MAX;
/// A node being walked, to stop a cycle in corrupt data.
const VISITING: u32 = u32::MAX - 1;
/// A node whose owner is not worked out yet.
const UNSET: u32 = u32::MAX - 2;
/// How many scopes are remembered. The HUD has about fifteen; a theme that keeps making new ones starts over.
const MAX_SCOPES: usize = 48;

/// One node as the cache saw it.
type Shape = (usize, u32, Option<usize>);

pub(crate) struct Structure {
    shape: Vec<Shape>,
    /// The generation of the frame cell this was last compared to the tree at.
    pub(super) verified: u64,
    /// For each node, the position of its parent in the tree, or [`NONE`].
    pub(super) parents: Vec<u32>,
    /// For each scope asked, the position of the owning root of each node, or [`NONE`].
    pub(super) owners: Vec<(Scope, Vec<u32>)>,
    /// Where each kept node lands in the list being built (`emit`'s scratch, kept to avoid an allocation per slot).
    pub(super) placed: Vec<u32>,
}

impl Structure {
    pub(crate) fn new(tree: &UiTree) -> Self {
        let shape = Self::shape_of(tree);
        // The last node with an index wins, as a lookup by index has always done.
        let by_index: HashMap<usize, usize> =
            tree.nodes.iter().enumerate().map(|(pos, node)| (node.index, pos)).collect();
        let parents = tree
            .nodes
            .iter()
            .map(|node| node.parent.and_then(|parent| by_index.get(&parent)).map_or(NONE, |at| *at as u32))
            .collect();
        Self { shape, verified: 0, parents, owners: Vec::new(), placed: Vec::new() }
    }

    fn shape_of(tree: &UiTree) -> Vec<Shape> {
        tree.nodes.iter().map(|node| (node.index, node.name_hash, node.parent)).collect()
    }

    /// Whether `tree` has the shape this was worked out for.
    pub(super) fn matches(&self, tree: &UiTree) -> bool {
        self.shape.len() == tree.nodes.len()
            && self.shape.iter().zip(&tree.nodes).all(|(s, n)| *s == (n.index, n.name_hash, n.parent))
    }

    /// Which entry of `owners` holds the answer for `scope`, working it out the first time.
    pub(crate) fn owner_slot(&mut self, tree: &UiTree, scope: &Scope) -> usize {
        if let Some(at) = self.owners.iter().position(|(s, _)| s == scope) {
            return at;
        }
        if self.owners.len() >= MAX_SCOPES {
            self.owners.clear();
        }
        let owners = match scope {
            Scope::Package => Vec::new(),
            Scope::Under { roots, excluded } => self.work_out(tree, roots, excluded),
        };
        self.owners.push((scope.clone(), owners));
        self.owners.len() - 1
    }

    /// The owning root of every node, in one pass over the parent links with each answer remembered.
    fn work_out(&self, tree: &UiTree, roots: &[u32], excluded: &[u32]) -> Vec<u32> {
        let count = tree.nodes.len();
        let mut owner = vec![UNSET; count];
        let mut chain: Vec<usize> = Vec::new();
        for start in 0..count {
            if owner[start] != UNSET {
                continue;
            }
            chain.clear();
            let mut at = start;
            // The answer for the end of the chain: the first root or excluded node on the way up decides it.
            let answer = loop {
                match owner[at] {
                    UNSET => {}
                    VISITING => break NONE,
                    known => break known,
                }
                let name = tree.nodes[at].name_hash;
                if excluded.contains(&name) {
                    owner[at] = NONE;
                    break NONE;
                }
                if roots.contains(&name) {
                    owner[at] = at as u32;
                    break at as u32;
                }
                owner[at] = VISITING;
                chain.push(at);
                match self.parents[at] {
                    NONE => break NONE,
                    parent => at = parent as usize,
                }
            };
            for &done in &chain {
                owner[done] = answer;
            }
        }
        owner
    }

    /// Whether the scope at `slot` takes the node at `pos`: it has an owner and the owner is visible.
    pub(crate) fn keeps(&self, tree: &UiTree, slot: usize, pos: usize) -> bool {
        let root = self.owners[slot].1[pos];
        root != NONE && tree.nodes[root as usize].visible
    }
}

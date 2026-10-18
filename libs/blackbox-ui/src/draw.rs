//! The draw list: the flat output of a component tree. It collects `UiNode`s in insertion order and turns them
//! into the `UiTree` a presenter draws.

use crate::tree::{UiNode, UiTree};

/// A growing list of draw nodes, filled by `Component::build`.
#[derive(Clone, Debug, Default)]
pub struct DrawList {
    nodes: Vec<UiNode>,
}

impl DrawList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, node: UiNode) {
        self.nodes.push(node);
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    /// The nodes in insertion order.
    pub fn iter(&self) -> std::slice::Iter<'_, UiNode> {
        self.nodes.iter()
    }

    /// The nodes sorted by `z`, stable, so ties keep insertion order. Lower `z` comes first.
    pub fn in_draw_order(&self) -> Vec<&UiNode> {
        let mut ordered: Vec<&UiNode> = self.nodes.iter().collect();
        ordered.sort_by(|a, b| a.z.total_cmp(&b.z));
        ordered
    }

    /// Turns the list into a `UiTree`. The nodes keep their insertion order, so parent indices stay valid.
    /// `drawn` picks the nodes that go into `draw_order`; the caller owns that policy (visibility, depth
    /// cut-off, which kinds draw). The order is farthest first (largest `z`), stable: later nodes draw on top.
    pub fn into_tree(self, drawn: impl Fn(&UiNode) -> bool) -> UiTree {
        let mut draw_order: Vec<usize> =
            self.nodes.iter().enumerate().filter(|(_, n)| drawn(n)).map(|(k, _)| k).collect();
        draw_order.sort_by(|a, b| self.nodes[*b].z.total_cmp(&self.nodes[*a].z));
        UiTree { nodes: self.nodes, draw_order }
    }

    /// Puts the nodes in `index` order, stable, so nodes that share an index keep their order. `parent` is a position
    /// in the list, so it is remapped to the node's new position. A parent that is not in the list becomes `None`.
    ///
    /// A composed list uses this to keep a source's object order, whatever order its slots were built in. Ties in `z`
    /// fall back to list order (`into_tree`), so the order matters for what draws on top.
    pub fn sort_by_index(&mut self) {
        let count = self.nodes.len();
        let mut order: Vec<usize> = (0..count).collect();
        order.sort_by_key(|&old| self.nodes[old].index);
        // `at[old]` is the new position of the node that was at `old`.
        let mut at = vec![0; count];
        for (new, &old) in order.iter().enumerate() {
            at[old] = new;
        }
        let mut taken: Vec<Option<UiNode>> = std::mem::take(&mut self.nodes).into_iter().map(Some).collect();
        self.nodes = order
            .iter()
            .filter_map(|&old| {
                let mut node = taken[old].take()?;
                node.parent = node.parent.and_then(|parent| at.get(parent).copied());
                Some(node)
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::DrawList;
    use crate::tree::UiNode;

    fn node(z: f32, colour: u8) -> UiNode {
        UiNode { colour: [colour, 0, 0, 255], z, visible: true, ..UiNode::default() }
    }

    #[test]
    fn starts_empty_and_counts_pushes() {
        let mut list = DrawList::new();
        assert!(list.is_empty());
        list.push(node(0.0, 1));
        list.push(node(0.0, 2));
        assert_eq!(list.len(), 2);
        list.clear();
        assert!(list.is_empty());
    }

    #[test]
    fn draw_order_sorts_by_z_and_keeps_ties_in_insertion_order() {
        let mut list = DrawList::new();
        list.push(node(2.0, 10));
        list.push(node(-1.0, 20));
        list.push(node(2.0, 30));
        list.push(node(0.0, 40));
        let order: Vec<u8> = list.in_draw_order().iter().map(|n| n.colour[0]).collect();
        assert_eq!(order, vec![20, 40, 10, 30]);
    }

    #[test]
    fn iter_keeps_insertion_order() {
        let mut list = DrawList::new();
        list.push(node(5.0, 1));
        list.push(node(-5.0, 2));
        let order: Vec<u8> = list.iter().map(|n| n.colour[0]).collect();
        assert_eq!(order, vec![1, 2]);
    }

    #[test]
    fn into_tree_keeps_every_node_and_orders_the_drawn_ones_farthest_first() {
        let mut list = DrawList::new();
        list.push(node(2.0, 10));
        list.push(UiNode { visible: false, ..node(9.0, 20) });
        list.push(node(2.0, 30));
        list.push(node(-1.0, 40));
        let tree = list.into_tree(|n| n.visible);
        assert_eq!(tree.nodes.len(), 4);
        assert_eq!(tree.nodes[1].colour[0], 20);
        // Node 1 is hidden, so it is left out; nodes 0 and 2 tie on z and keep insertion order.
        assert_eq!(tree.draw_order, vec![0, 2, 3]);
    }

    fn indexed(index: usize, parent: Option<usize>) -> UiNode {
        UiNode { index, parent, visible: true, z: 1.0, ..UiNode::default() }
    }

    #[test]
    fn sort_by_index_orders_nodes_and_remaps_parents() {
        // Built out of order: the child (index 2) comes first and points at the parent (index 5) by position 1.
        let mut list = DrawList::new();
        list.push(indexed(2, Some(1)));
        list.push(indexed(5, None));
        list.push(indexed(3, Some(1)));
        list.push(indexed(4, Some(9)));
        list.sort_by_index();
        let order: Vec<usize> = list.iter().map(|n| n.index).collect();
        assert_eq!(order, vec![2, 3, 4, 5]);
        // Node 2 now sits at position 0 and its parent (index 5) at position 3; node 4's parent is not in the list.
        assert_eq!(list.iter().map(|n| n.parent).collect::<Vec<_>>(), vec![Some(3), Some(3), None, None]);
    }

    #[test]
    fn sort_by_index_keeps_equal_indices_in_list_order() {
        let mut list = DrawList::new();
        list.push(UiNode { colour: [1, 0, 0, 255], ..indexed(0, None) });
        list.push(UiNode { colour: [2, 0, 0, 255], ..indexed(0, None) });
        list.push(UiNode { colour: [3, 0, 0, 255], ..indexed(0, None) });
        list.sort_by_index();
        let order: Vec<u8> = list.iter().map(|n| n.colour[0]).collect();
        assert_eq!(order, vec![1, 2, 3]);
    }

    #[test]
    fn into_tree_uses_the_caller_predicate() {
        let mut list = DrawList::new();
        list.push(node(1.0, 1));
        list.push(node(3.0, 2));
        let tree = list.into_tree(|n| n.z > 2.0);
        assert_eq!(tree.draw_order, vec![1]);
    }
}

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

    #[test]
    fn into_tree_uses_the_caller_predicate() {
        let mut list = DrawList::new();
        list.push(node(1.0, 1));
        list.push(node(3.0, 2));
        let tree = list.into_tree(|n| n.z > 2.0);
        assert_eq!(tree.draw_order, vec![1]);
    }
}

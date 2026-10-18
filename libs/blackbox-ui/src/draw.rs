//! The draw list: the flat output of a component tree, in draw order.

use crate::geom::Rect;

// T3: `DrawNode` is a placeholder. T3 moves the neutral tree (`UiTree`, `UiNode`, `NodeKind`) from
// `libs/blackbox-feng/src/tree/mod.rs` into this crate and makes `DrawList` hold `UiNode`. The
// `z: f32` draw order carries over unchanged, so `DrawList::in_draw_order` keeps its meaning.
// Until then nothing outside this crate reads `DrawNode`.

/// One primitive: a coloured rectangle at a draw order. Stands in for the relocated `UiNode`.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawNode {
    pub bounds: Rect,
    /// `[r, g, b, a]`.
    pub colour: [u8; 4],
    /// Draw order: lower draws first. Equal values keep insertion order.
    pub z: f32,
}

/// A growing list of draw nodes, filled by `Component::build`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawList {
    nodes: Vec<DrawNode>,
}

impl DrawList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, node: DrawNode) {
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
    pub fn iter(&self) -> std::slice::Iter<'_, DrawNode> {
        self.nodes.iter()
    }

    /// The nodes sorted by `z`, stable, so ties keep insertion order. This is the order a presenter draws.
    pub fn in_draw_order(&self) -> Vec<&DrawNode> {
        let mut ordered: Vec<&DrawNode> = self.nodes.iter().collect();
        ordered.sort_by(|a, b| a.z.total_cmp(&b.z));
        ordered
    }
}

#[cfg(test)]
mod tests {
    use super::{DrawList, DrawNode};
    use crate::geom::Rect;

    fn node(z: f32, colour: u8) -> DrawNode {
        DrawNode { bounds: Rect::SCREEN, colour: [colour, 0, 0, 255], z }
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
}

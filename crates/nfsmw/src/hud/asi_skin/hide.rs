//! Hiding the stock gauges while a skin draws its own, so no gauge is drawn twice.

use blackbox_ui::UiTree;

use crate::hud::roots;

/// The name hashes of the objects of the stock tachometer, speedometer, gear, nitrous, turbo and speed breaker.
fn stock_roots() -> Vec<u32> {
    [roots::tachometer(), roots::speedometer(), roots::gear(), roots::nitrous(), roots::turbo(), roots::speedbreaker()]
        .iter()
        .flat_map(|element| element.roots.iter().copied())
        .collect()
}

/// Hides every node that is one of `roots` or has one among its ancestors, and takes it out of the draw order.
pub(super) fn hide_under(tree: &mut UiTree, roots: &[u32]) {
    let count = tree.nodes.len();
    let mut hidden = vec![false; count];
    for (at, slot) in hidden.iter_mut().enumerate() {
        let mut current = Some(at);
        // A parent chain is shorter than the tree; the bound stops a malformed one.
        for _ in 0..count {
            let Some(node) = current.and_then(|index| tree.nodes.get(index)) else { break };
            if roots.contains(&node.name_hash) {
                *slot = true;
                break;
            }
            current = node.parent;
        }
    }
    for (node, hide) in tree.nodes.iter_mut().zip(&hidden) {
        if *hide {
            node.visible = false;
        }
    }
    let nodes = &tree.nodes;
    tree.draw_order.retain(|&index| nodes.get(index).is_some_and(|node| node.visible));
}

/// Hides the stock gauges of the HUD tree.
pub(crate) fn hide_stock(tree: &mut UiTree) {
    hide_under(tree, &stock_roots());
}

#[cfg(test)]
mod tests {
    use blackbox_ui::UiNode;

    use super::*;

    fn node(name_hash: u32, parent: Option<usize>) -> UiNode {
        UiNode { name_hash, parent, visible: true, ..UiNode::default() }
    }

    #[test]
    fn a_root_and_everything_under_it_is_hidden_and_leaves_the_draw_order() {
        // 0 minimap group, 1 tachometer root (hash 7), 2 its needle, 3 a nested label, 4 another tree's node.
        let mut tree = UiTree {
            nodes: vec![node(1, None), node(7, None), node(2, Some(1)), node(3, Some(2)), node(4, Some(0))],
            draw_order: vec![0, 1, 2, 3, 4],
        };
        hide_under(&mut tree, &[7]);
        let visible: Vec<bool> = tree.nodes.iter().map(|n| n.visible).collect();
        assert_eq!(visible, vec![true, false, false, false, true]);
        assert_eq!(tree.draw_order, vec![0, 4]);
    }

    #[test]
    fn a_tree_with_no_stock_gauge_is_untouched() {
        let mut tree = UiTree { nodes: vec![node(1, None), node(2, Some(0))], draw_order: vec![0, 1] };
        hide_under(&mut tree, &[7, 8]);
        assert_eq!(tree.draw_order, vec![0, 1]);
        assert!(tree.nodes.iter().all(|n| n.visible));
    }

    #[test]
    fn a_parent_cycle_ends() {
        let mut tree = UiTree { nodes: vec![node(1, Some(1)), node(2, Some(0))], draw_order: vec![0, 1] };
        hide_under(&mut tree, &[7]);
        assert_eq!(tree.draw_order, vec![0, 1]);
    }

    #[test]
    fn the_stock_roots_are_the_six_gauge_groups() {
        let roots = stock_roots();
        assert!(!roots.is_empty());
        assert!(roots.contains(&roots::gear().roots[0]));
        assert!(roots.contains(&roots::speedbreaker().roots[0]));
    }
}

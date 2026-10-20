//! `Group`: parts drawn together at one opacity, so a whole piece of the map can fade in or slide as one.

use blackbox_ui::{Component, DrawList, LayoutCx, Rect};

/// The parts of a piece of UI and the opacity they are drawn at, 0 to 1. The opacity multiplies each node's alpha.
pub struct Group {
    parts: Vec<Box<dyn Component>>,
    opacity: f32,
}

impl Group {
    pub fn new(parts: Vec<Box<dyn Component>>, opacity: f32) -> Self {
        Self { parts, opacity: opacity.clamp(0.0, 1.0) }
    }
}

impl Component for Group {
    /// The parts are laid out when they are made, so there is nothing to place here.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        if self.opacity >= 1.0 {
            for part in &self.parts {
                part.build(out);
            }
            return;
        }
        let mut inner = DrawList::new();
        for part in &self.parts {
            part.build(&mut inner);
        }
        for node in inner.iter() {
            let mut node = node.clone();
            node.colour[3] = faded(node.colour[3], self.opacity);
            node.world_colour[3] = faded(node.world_colour[3], self.opacity);
            out.push(node);
        }
    }
}

fn faded(alpha: u8, opacity: f32) -> u8 {
    (f32::from(alpha) * opacity).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_ui::{Panel, Skin};

    fn group(opacity: f32) -> DrawList {
        let skin = Skin::new().with_colour("c", [10, 20, 30, 200]);
        let mut panel = Panel::new("c");
        panel.layout(&LayoutCx::new(&skin, 0.0), Rect::new(0.0, 0.0, 4.0, 4.0));
        let group = Group::new(vec![Box::new(panel)], opacity);
        let mut out = DrawList::new();
        group.build(&mut out);
        out
    }

    #[test]
    fn a_group_scales_the_alpha_of_every_node_and_leaves_the_colour() {
        let out = group(0.5);
        let node = out.iter().next().expect("one node");
        assert_eq!((node.colour, node.world_colour), ([10, 20, 30, 100], [10, 20, 30, 100]));
    }

    #[test]
    fn a_group_at_full_opacity_is_the_parts_as_they_are() {
        assert_eq!(group(1.0).iter().next().expect("one node").colour[3], 200);
        assert_eq!(group(2.0).iter().next().expect("one node").colour[3], 200, "clamped to 1");
        assert_eq!(group(0.0).iter().next().expect("one node").colour[3], 0);
    }
}

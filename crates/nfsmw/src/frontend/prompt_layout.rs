//! Replacement hints inherit a package's animated footer, before its legacy hints are hidden.

use blackbox_feng::{UiNode, UiTree};
use glam::{Mat4, Vec3};

use super::ids::screen;

pub struct Frame {
    pub world: Mat4,
    pub width: f32,
    pub height: f32,
    pub centred: bool,
    parent: usize,
    opacity: u8,
    visible: bool,
}

impl Frame {
    pub fn find(tree: &UiTree, name: &str) -> Option<Self> {
        if [screen::SPLASH, screen::SPLASH_WIDE].iter().any(|s| name.eq_ignore_ascii_case(s)) {
            let n = tree.nodes.iter().find(|n| n.name_hash == 0xC4DF_3FF2)?;
            return Some(Self::new(n, n.world, 220.0, 32.0, true));
        }
        // These groups own the footer fade; the image supplies its rectangle, not its low background alpha.
        let (group, background) = match name.to_ascii_lowercase().as_str() {
            "pause_main.fng" => (0x2FE7_4244, 0x81B4_FAD0),
            "pause_options.fng" => (0xB3A6_9E89, 0x81B4_FAD0),
            "options.fng" => (0x84BC_F2B5, 0x3D3B_C1AC),
            "mainmenu.fng" | "mainmenu_sub.fng" => (0x17B1_F254, 0xC6AF_DD7E),
            _ => return None,
        };
        let root = tree.nodes.iter().find(|n| n.name_hash == group)?;
        let bar = tree.nodes.iter().find(|n| n.parent == Some(root.index) && n.name_hash == background)?;
        let world = root.world * Mat4::from_translation(bar.local_position.truncate().extend(0.0));
        Some(Self::new(root, world, bar.size.x.abs() - 12.0, bar.size.y.abs(), false))
    }

    fn new(root: &UiNode, world: Mat4, width: f32, height: f32, centred: bool) -> Self {
        Self { world, width, height, centred, parent: root.index, opacity: root.world_colour[3], visible: root.visible }
    }

    pub fn apply(&self, tree: &mut UiTree, first: usize) {
        for n in &mut tree.nodes[first..] {
            n.parent = Some(self.parent);
            n.world = self.world * n.world;
            n.position = n.world.transform_point3(Vec3::ZERO);
            n.z = n.position.z;
            n.world_colour[3] = (u16::from(n.world_colour[3]) * u16::from(self.opacity) / 255) as u8;
            n.visible = self.visible && n.world_colour[3] != 0 && n.z > 0.0;
        }
        tree.draw_order.retain(|i| *i < first || tree.nodes[*i].visible);
    }
}

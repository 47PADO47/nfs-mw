//! Replacement hints inherit a package's animated footer, before its legacy hints are hidden.

use blackbox_feng::{UiNode, UiTree};
use glam::{Mat4, Vec3};

use super::ids::screen;
use super::logic::Footer;

/// The group of `Quick_Race_Brief.fng` that holds the Back and Accept hints, which the footer replaces.
pub const BRIEF_FOOTER: u32 = 0x03CA_06E1;
/// The group of `Quick_Race_Brief.fng` that holds the Randomize hint, which the footer replaces.
pub const BRIEF_RANDOMIZE: u32 = 0x02DD_F58C;

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
    pub fn find(tree: &UiTree, name: &str, footer: Footer) -> Option<Self> {
        if footer.is_garage() {
            let main = name.eq_ignore_ascii_case("CustomizeMain.fng")
                || name.to_ascii_lowercase().starts_with("customizecategory");
            let groups = match main {
                true => [0x17b1_f254, 0x84bc_f2b5, 0xb3a6_9e89, 0xc0a3_2823],
                false => [0x84bc_f2b5, 0xb3a6_9e89, 0x17b1_f254, 0xc0a3_2823],
            };
            let root = groups
                .into_iter()
                .find_map(|hash| tree.nodes.iter().find(|node| node.name_hash == hash && node.world.w_axis.z > 0.0))?;
            let at = Vec3::new(0.0, root.world.w_axis.y, root.world.w_axis.z);
            return Some(Self::new(root, Mat4::from_translation(at), 590.0, 26.0, false));
        }
        if [screen::SPLASH, screen::SPLASH_WIDE].iter().any(|s| name.eq_ignore_ascii_case(s)) {
            let n = tree.nodes.iter().find(|n| n.name_hash == 0xC4DF_3FF2)?;
            return Some(Self::new(n, n.world, 220.0, 32.0, true));
        }
        // These groups own the footer fade; the image supplies its rectangle, not its low background alpha.
        let (group, background) = match name.to_ascii_lowercase().as_str() {
            "pause_main.fng" => (0x2FE7_4244, 0x81B4_FAD0),
            "pause_options.fng" => (0xB3A6_9E89, 0x81B4_FAD0),
            "options.fng" => (0x84BC_F2B5, 0x3D3B_C1AC),
            "safehousereputationoverview.fng" => (0x84BC_F2B5, 0x3D3B_C1AC),
            // The custom race's screens (`custom_race/`); the career's car lot keeps the package's own footer.
            "track_select.fng" | "track_options.fng" | "showcase.fng" => (0x84BC_F2B5, 0x3D3B_C1AC),
            "car_select.fng" if footer.is_custom_car_select() => (0x84BC_F2B5, 0x3D3B_C1AC),
            "mainmenu.fng" | "mainmenu_sub.fng" | "mc_profilemanager.fng" => (0x17B1_F254, 0xC6AF_DD7E),
            // The profile list's footer group is empty and sits below its panel: the hints go where its Back button was,
            // inside the panel's footer.
            "mc_list.fng" => {
                let root = tree.nodes.iter().find(|n| n.name_hash == super::ids::QUIT_BUTTON)?;
                return Some(Self::new(root, root.world, 150.0, 26.0, false));
            }
            // The Quick Play brief's footer has no bar of its own to centre on: Back and Accept share one group and
            // Randomize has another, none with a box a click can hit. The hints go on the Back group's line, centred.
            "quick_race_brief.fng" => {
                let root = tree.nodes.iter().find(|n| n.name_hash == BRIEF_FOOTER)?;
                let at = Vec3::new(0.0, root.world.w_axis.y, root.world.w_axis.z);
                return Some(Self::new(root, Mat4::from_translation(at), 540.0, 26.0, true));
            }
            _ => return None,
        };
        let root = tree.nodes.iter().find(|n| n.name_hash == group)?;
        let bar = tree.nodes.iter().find(|n| n.name_hash == background && is_inside(tree, n, root.index))?;
        // Where the bar's origin is in the group's space: its own offset for a child, the sum of the offsets for a deeper one.
        let at = root.world.inverse().transform_point3(bar.world.w_axis.truncate());
        let world = root.world * Mat4::from_translation(at.truncate().extend(0.0));
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

/// Whether `node` is `root` or inside it, following the parents.
fn is_inside(tree: &UiTree, node: &UiNode, root: usize) -> bool {
    let mut parent = node.parent;
    for _ in 0..tree.nodes.len() {
        match parent {
            Some(p) if p == root => return true,
            Some(p) => parent = tree.nodes.get(p).and_then(|n| n.parent),
            None => return false,
        }
    }
    false
}

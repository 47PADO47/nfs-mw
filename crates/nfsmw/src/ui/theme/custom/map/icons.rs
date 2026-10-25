//! The game's own icons for the marks and the player's arrow: the textures of the HUD package's minimap objects
//! (`docs/specs/hud-minimap.md` section 7, `hud/blip_styles.rs`), drawn at the Custom map's projected places.
//!
//! The icons are found in the frame's HUD tree by their object names. An icon the package lacks (a different HUD, or a
//! kind with no icon of the game's, like a roadblock) is drawn as its shape (`draw.rs`).

use blackbox_feng::fe_hash_upper;
use blackbox_ui::{NodeKind, UiTree};
use glam::Vec2;

use super::marks::{CAR_LOT, MarkKind};
use crate::hud::{AUTO_SHOP_TINT, CAR_LOT_TINT, SAFE_HOUSE_TINT, cop_colour, cop_step_at};

/// The player's arrow (`Minimap_Icon_Car`).
pub const PLAYER_ARROW: &str = "PLAYERCARINDICATOR";
/// The checkpoint's icon, which the GPS route's end uses too (`GpsTarget`).
const CHECKPOINT: &str = "MMICON_CHECKPOINT";
/// The race events' icons, `<prefix>0`, by the race type's number (`RaceType`).
const RACE_ICONS: [&str; 6] =
    ["MMICON_SPRINT_", "MMICON_CIRCUIT_", "MMICON_DRAG_", "MMICON_KNOCKOUT_", "MMICON_TOLLBOOTH_", "MMICON_SPEEDTRAP_"];
/// How many racers have an icon of their own.
const RACER_ICONS: u8 = 8;
/// How far down a group the icon's image may be.
const DEPTH: usize = 4;

/// An icon of the package: its texture, its size in HUD units and the colour the package gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Icon {
    pub texture: u32,
    pub size: Vec2,
    pub colour: [u8; 4],
}

/// The object a mark of `kind` and `variant` draws, or `None` for a kind with no icon of the game's.
pub fn object(kind: MarkKind, variant: u8) -> Option<String> {
    match kind {
        MarkKind::NextCheckpoint | MarkKind::Checkpoint | MarkKind::Target => Some(CHECKPOINT.to_owned()),
        MarkKind::SpeedTrap => Some(format!("{}0", RACE_ICONS[5])),
        MarkKind::Racer => Some(format!("MMICON_AIRACER_{}", variant % RACER_ICONS)),
        MarkKind::Race => Some(format!("{}0", RACE_ICONS.get(usize::from(variant)).unwrap_or(&RACE_ICONS[0]))),
        MarkKind::SafeHouse => Some("SAFEHOUSEINDICATOR_0".to_owned()),
        MarkKind::Shop => Some(match variant == CAR_LOT {
            true => "CAR_LOT_0".to_owned(),
            false => "SHOP_0".to_owned(),
        }),
        MarkKind::Cop => Some("MMICON_COPCAR_0".to_owned()),
        MarkKind::Helicopter => Some("HELICOPTER_ICON_GROUP".to_owned()),
        MarkKind::LastKnown | MarkKind::Roadblock => None,
    }
}

/// The colour a mark's icon takes when the package's own is plain white: the game's tint of the interiors, and the
/// light bar of a cop at `tick`.
pub fn tint(kind: MarkKind, variant: u8, tick: u32) -> Option<[u8; 4]> {
    match kind {
        MarkKind::Shop if variant == CAR_LOT => Some(CAR_LOT_TINT),
        MarkKind::Shop => Some(AUTO_SHOP_TINT),
        MarkKind::SafeHouse => Some(SAFE_HOUSE_TINT),
        MarkKind::Cop => Some(cop_colour(true, cop_step_at(tick))),
        _ => None,
    }
}

/// The icon of the object `name` in `tree`: the object's own image, or the first image under it when the object is a
/// group. `None` when the package has no such object, or it draws no picture.
pub fn find(tree: &UiTree, name: &str) -> Option<Icon> {
    let hash = fe_hash_upper(name);
    let mut node = tree.nodes.iter().find(|node| node.name_hash == hash)?;
    for _ in 0..DEPTH {
        if let NodeKind::Image { texture, .. } = node.kind
            && texture != 0
        {
            let size = Vec2::new(node.world.x_axis.truncate().length(), node.world.y_axis.truncate().length());
            return (size.x > 0.0 && size.y > 0.0).then_some(Icon { texture, size, colour: node.colour });
        }
        node = tree.nodes.iter().find(|child| child.parent == Some(node.index))?;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_ui::UiNode;
    use glam::{Mat4, Vec3};

    fn image(index: usize, parent: Option<usize>, name: &str, texture: u32) -> UiNode {
        UiNode {
            index,
            parent,
            name_hash: fe_hash_upper(name),
            kind: NodeKind::Image {
                texture,
                uv: [0.0, 0.0, 1.0, 1.0],
                mask: None,
                mask_rotation: [0.5, 0.5, 0.0],
                mask_uv: [0.0, 0.0, 1.0, 1.0],
            },
            world: Mat4::from_scale(Vec3::new(14.0, 16.0, 1.0)),
            colour: [255, 200, 0, 255],
            ..UiNode::default()
        }
    }

    fn group(index: usize, name: &str) -> UiNode {
        UiNode { index, name_hash: fe_hash_upper(name), ..UiNode::default() }
    }

    #[test]
    fn an_image_object_gives_its_texture_size_and_colour() {
        let tree = UiTree { nodes: vec![image(0, None, "MMICON_CHECKPOINT", 77)], draw_order: Vec::new() };
        let icon = find(&tree, "MMICON_CHECKPOINT").expect("the icon");
        assert_eq!((icon.texture, icon.size, icon.colour), (77, Vec2::new(14.0, 16.0), [255, 200, 0, 255]));
    }

    #[test]
    fn a_group_gives_the_image_under_it() {
        let tree = UiTree {
            nodes: vec![group(3, "HELICOPTER_ICON_GROUP"), image(4, Some(3), "HELI_IMAGE", 9)],
            draw_order: Vec::new(),
        };
        assert_eq!(find(&tree, "HELICOPTER_ICON_GROUP").map(|icon| icon.texture), Some(9));
    }

    #[test]
    fn a_missing_object_or_a_group_without_an_image_has_no_icon() {
        let tree = UiTree { nodes: vec![group(0, "EMPTY_GROUP")], draw_order: Vec::new() };
        assert_eq!(find(&tree, "EMPTY_GROUP"), None);
        assert_eq!(find(&tree, "NOT_THERE"), None);
        let flat = UiTree { nodes: vec![image(0, None, "FLAT", 5)], draw_order: Vec::new() };
        let zero = UiNode { world: Mat4::ZERO, ..flat.nodes[0].clone() };
        assert_eq!(find(&UiTree { nodes: vec![zero], draw_order: Vec::new() }, "FLAT"), None, "no size, no icon");
    }

    #[test]
    fn each_kind_names_its_object() {
        assert_eq!(object(MarkKind::Target, 0).as_deref(), Some("MMICON_CHECKPOINT"));
        assert_eq!(object(MarkKind::Racer, 9).as_deref(), Some("MMICON_AIRACER_1"));
        assert_eq!(object(MarkKind::Race, 2).as_deref(), Some("MMICON_DRAG_0"));
        assert_eq!(
            object(MarkKind::Race, 200).as_deref(),
            Some("MMICON_SPRINT_0"),
            "an unknown type uses the sprint's"
        );
        assert_eq!(object(MarkKind::Shop, CAR_LOT).as_deref(), Some("CAR_LOT_0"));
        assert_eq!(object(MarkKind::Shop, 0).as_deref(), Some("SHOP_0"));
        assert_eq!(object(MarkKind::Roadblock, 0), None);
        assert_eq!(object(MarkKind::LastKnown, 0), None);
    }
}

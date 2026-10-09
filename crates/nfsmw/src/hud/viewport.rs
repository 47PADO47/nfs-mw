//! HUD-only viewport presets, from `docs/specs/hud-viewport.md`.
//! The translation follows each node's rotation, keeping the map centered through turns.

use blackbox_feng::{UiTree, fe_hash_upper};
use glam::{Mat4, Vec3};

use super::minimap::{ARROW, GROUP};
use crate::settings::HudLayout;
use crate::ui::present::Screen;

/// The contexts moved by the package's WIDESCREENMODE scripts.
const LEFT_CONTEXT: u32 = 0x1603_009e;
const RIGHT_CONTEXT: u32 = 0x5d01_01f1;
const NORMAL_ASPECT: f32 = 4.0 / 3.0;
const WIDE_ASPECT: f32 = 16.0 / 9.0;
const STOCK_SHIFT: f32 = 120.0;
const XBOX_SCALE: f32 = 0.92;

/// The side of each package object, cached once; no live runtime positions are modified.
pub(super) struct HudViewport {
    sides: Vec<i8>,
}

impl HudViewport {
    pub fn new(tree: &UiTree) -> Self {
        let sides = tree.nodes.iter().map(|node| side(tree, node.index)).collect();
        Self { sides }
    }

    /// A viewport that moves every node of `tree` as the HUD's left side moves (a package that is all on the left).
    pub fn all_left(tree: &UiTree) -> Self {
        Self { sides: vec![-1; tree.nodes.len()] }
    }

    /// Applies a preset to a fresh runtime tree. Menus never pass through this viewport.
    pub fn apply(&self, tree: &mut UiTree, screen: Screen, layout: HudLayout) {
        let (scale, offset) = parameters(screen, layout);
        if scale == 1.0 && offset == 0.0 {
            return;
        }
        let scaled = Mat4::from_scale(Vec3::new(scale, scale, 1.0));
        for node in &mut tree.nodes {
            let side = self.sides.get(node.index).copied().unwrap_or(0);
            let transform = scaled * Mat4::from_translation(Vec3::new(f32::from(side) * offset, 0.0, 0.0));
            node.world = transform * node.world;
            node.position = transform.transform_point3(node.position);
        }
    }
}

fn side(tree: &UiTree, mut index: usize) -> i8 {
    for _ in 0..tree.nodes.len() {
        let Some(node) = tree.nodes.get(index) else { return 0 };
        match node.name_hash {
            LEFT_CONTEXT => return -1,
            RIGHT_CONTEXT => return 1,
            hash if hash == fe_hash_upper(GROUP) || hash == fe_hash_upper(ARROW) => return -1,
            _ => {}
        }
        let Some(parent) = node.parent else { return 0 };
        index = parent;
    }
    0
}

/// Stock 16:9 placement; other aspect ratios interpolate/extend it without stretching.
pub(super) fn parameters(screen: Screen, layout: HudLayout) -> (f32, f32) {
    if layout == HudLayout::Classic || !screen.width.is_finite() || !screen.height.is_finite() {
        return (1.0, 0.0);
    }
    if screen.width <= 0.0 || screen.height <= 0.0 {
        return (1.0, 0.0);
    }
    let aspect = screen.width / screen.height;
    if aspect <= NORMAL_ASPECT {
        return (1.0, 0.0);
    }
    let scale = match layout {
        HudLayout::Xbox360 => XBOX_SCALE,
        _ => 1.0,
    };
    let progress = ((aspect - NORMAL_ASPECT) / (WIDE_ASPECT - NORMAL_ASPECT)).clamp(0.0, 1.0);
    let offset = STOCK_SHIFT * progress + 240.0 * (aspect - WIDE_ASPECT).max(0.0) / scale;
    (scale, offset)
}

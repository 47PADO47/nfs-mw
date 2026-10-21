//! Where a split-screen HUD lands in its half of the window (`docs/specs/split-screen-hud.md` section 3).
//!
//! The four packages are authored on the whole 640 x 480 canvas, each in its own half of it: the first player's
//! elements have y between -240 and 0, the second player's between 0 and 240 **[verified]**. So there is no per-half
//! canvas: each package is drawn with the ordinary HUD math (the canvas fills the window height, the widescreen
//! contexts move out by the same offset as in `HUD_SingleRace.fng`) and lands in its half by itself. The only additions
//! are the contexts the split packages have besides the two of the single-player package, a scale that keeps a window
//! narrower than 4:3 from clipping the 640-unit-wide layout, and the rule between the halves, which spans the window.

use blackbox_feng::{PackageId, Runtime, UiTree};
use glam::{Mat4, Vec3};

use super::Seat;
use super::names::DIVIDER_GUID;
use crate::hud::viewport::HudViewport;
use crate::settings::HudLayout;
use crate::ui::present::Screen;

/// The widescreen message of the packages (`WIDESCREENMODE`): the contexts that answer it move out to their side.
const WIDESCREEN_MODE: u32 = 0x62ED_04EC;
/// The left widescreen context, which moves the other way.
const LEFT_CONTEXT: u32 = 0x1603_009E;
/// The canvas is 640 units wide at the 4:3 the layout was made for.
const CANVAS_ASPECT: f32 = 4.0 / 3.0;

#[allow(dead_code)] // for the app that draws the world pictures (`app/split.rs`)
/// The part of the window that is a player's: the whole width, half the height, the first player above the second
/// (`[x, y, width, height]`, in points).
pub fn half_rect(seat: Seat, screen: Screen) -> [f32; 4] {
    let half = screen.height / 2.0;
    [0.0, half * seat.index() as f32, screen.width, half]
}

/// The scale that keeps the 640-unit-wide layout inside a window narrower than 4:3: 1 at 4:3 and wider.
pub fn fit(screen: Screen) -> f32 {
    if !(screen.width.is_finite() && screen.height.is_finite()) || screen.width <= 0.0 || screen.height <= 0.0 {
        return 1.0;
    }
    (screen.width / screen.height / CANVAS_ASPECT).min(1.0)
}

/// The placement of one package, built once.
pub struct Placement {
    viewport: HudViewport,
}

impl Placement {
    /// `blip_icons` are the name hashes of the map's extra arrows, which move with the minimap's side.
    pub fn new(rt: &Runtime, package: PackageId, blip_icons: &[u32]) -> Self {
        let contexts = contexts(rt, package);
        Self { viewport: HudViewport::with_contexts(&rt.tree(package), &contexts, blip_icons) }
    }

    /// Places a fresh runtime tree for `screen` under a layout preset.
    pub fn apply(&self, tree: &mut UiTree, screen: Screen, layout: HudLayout) {
        self.viewport.apply(tree, screen, layout);
        let fit = fit(screen);
        if fit < 1.0 {
            let scale = Mat4::from_scale(Vec3::new(fit, fit, 1.0));
            for node in &mut tree.nodes {
                node.world = scale * node.world;
                node.position = scale.transform_point3(node.position);
            }
        }
        span_window(tree, screen);
    }
}

/// The objects that answer the widescreen message, with the side they move to: the left context to the left, any other to
/// the side its position is on. The first player's gauges are in a context of their own name (`0x2FE4FF9C`).
fn contexts(rt: &Runtime, package: PackageId) -> Vec<(u32, i8)> {
    let Some(def) = rt.package(package) else { return Vec::new() };
    let Some(targets) = def.targets.iter().find(|t| t.message == WIDESCREEN_MODE) else { return Vec::new() };
    let mut out: Vec<(u32, i8)> = Vec::new();
    for guid in &targets.guids {
        let Some(object) = def.objects.iter().find(|o| o.guid == *guid) else { continue };
        let side = match (object.name_hash == LEFT_CONTEXT, object.data.position().x < 0.0) {
            (true, _) | (_, true) => -1,
            _ => 1,
        };
        if !out.iter().any(|(hash, _)| *hash == object.name_hash) {
            out.push((object.name_hash, side));
        }
    }
    out
}

/// Widens the rule between the halves (an image 1200 units wide in the first player's package) to the window when the
/// window is wider than that. It is centred on the middle of the canvas.
fn span_window(tree: &mut UiTree, screen: Screen) {
    let Some(node) = tree.nodes.iter_mut().find(|n| n.guid == DIVIDER_GUID) else { return };
    let drawn = node.world.x_axis.length();
    let scale = screen.scale();
    if drawn <= 0.0 || scale <= 0.0 {
        return;
    }
    // The window's width in canvas units.
    let wanted = screen.width / scale;
    if wanted <= drawn {
        return;
    }
    let k = wanted / drawn;
    node.world = Mat4::from_scale(Vec3::new(k, 1.0, 1.0)) * node.world;
}

//! The Remastered minimap: the game's own map pieces in a dark round window, a thin bezel with a gold hairline, the north
//! marker on it, one ring of ten segments attached to its outside for the heat, and a flame with the heat multiplier at its top left. The
//! chase's own state (busted, escaping, cool-down) is the pursuit panel's, not the minimap's. The map pieces, the player's
//! arrow and the position and turn of the window come from the HUD package (the same objects the Original look draws); the
//! rest is drawn here.
//!
//! It fills the `Minimap` slot, and claims the package's heat group with it (`hud::roots::minimap`), so the original
//! flame, multiplier and arcs do not draw beside it.

#[cfg(test)]
mod tests;
mod tint;

use blackbox_feng::fe_hash_upper;
use blackbox_ui::{Component, DrawList, Image, LayoutCx, NodeKind, Rect, Ring, Rounded, Text, UiTree, justify};
use glam::Vec3;

use self::tint::Tinted;
use super::group::Group;
use super::skin::keys;
use crate::hud::minimap::{BACKING, GROUP};
use crate::hud::roots;
use crate::ui::theme::custom::DriveFeedCell;
use crate::ui::theme::original::{FengSource, FengSubtree, FrameCell};

/// The window's radius in FEng units: the mask texture covers 128 units, and the circle in it is as wide.
const MAP_RADIUS: f32 = 64.0;
/// The bezel: a dark ring on the edge of the window and a gold hairline inside it.
const BEZEL_WIDTH: f32 = 2.0;
const HAIRLINE_WIDTH: f32 = 0.7;
/// The heat ring: ten segments, each a little shorter than its tenth of the circle, outside the bezel and touching it.
const HEAT_SEGMENTS: u32 = 10;
const HEAT_GAP: f32 = 7.0;
const HEAT_OFFSET: f32 = BEZEL_WIDTH + HEAT_WIDTH / 2.0;
const HEAT_WIDTH: f32 = 1.8;
/// The north marker's badge and text.
const NORTH_BADGE: f32 = 12.0;
const NORTH_SCALE: f32 = 1.25;
/// The flame and the multiplier: up and to the left of the window, at this angle (clockwise from up) and this far from
/// its edge.
const FLAME_ANGLE: f32 = 315.0;
const FLAME_OFFSET: f32 = 17.0;
const FLAME_SIZE: f32 = 17.0;
const HEAT_TEXT_SCALE: f32 = 1.5;
const HEAT_TEXT_WIDTH: f32 = 60.0;
/// How much nearer to the viewer than the map pieces the bezel, the badge and the labels are, and how much farther the dark
/// disc under them is.
const ABOVE_MAP: f32 = 50.0;
const BEHIND_MAP: f32 = 20.0;
/// The package's own heat icon (`Heat_Icon.tga`, by its object name), whose texture the flame reuses.
const FLAME_OBJECT: u32 = 0xC30E_A0A9;

/// The minimap, built from the HUD package's tree and the drive feed.
pub struct Minimap {
    frame: FrameCell,
    feed: DriveFeedCell,
    parts: Vec<Box<dyn Component>>,
}

impl Minimap {
    pub fn new(frame: FrameCell, feed: DriveFeedCell) -> Self {
        Self { frame, feed, parts: Vec::new() }
    }
}

impl Component for Minimap {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        self.parts.clear();
        let Some(tree) = self.frame.get(FengSource::Hud) else { return };
        let Some(window) = Window::of(&tree) else { return };
        let parts = parts(&self.frame, &self.feed, &tree, &window, cx);
        self.parts = vec![Box::new(Group::new(parts, window.opacity))];
    }

    fn build(&self, out: &mut DrawList) {
        for part in &self.parts {
            part.build(out);
        }
    }
}

/// Where the window is on the screen. The group of the map pieces (`TRACK_MAP`) scrolls under the window as the car moves and
/// turns with its heading, so its turn gives the direction of north; the window itself stays where the backing disc is.
pub(super) struct Window {
    pub centre: (f32, f32),
    pub radius: f32,
    /// The unit vector from the centre towards the map's north, on the screen.
    pub north: (f32, f32),
    pub depth: f32,
    /// The group's own opacity, 0 to 1: the HUD fades it in and out.
    pub opacity: f32,
}

impl Window {
    /// The window of `tree`, or `None` while the group or the backing disc is absent or hidden.
    pub(super) fn of(tree: &UiTree) -> Option<Self> {
        let find = |name: &str| tree.nodes.iter().find(|node| node.name_hash == fe_hash_upper(name));
        let (group, backing) = (find(GROUP)?, find(BACKING)?);
        if !group.visible {
            return None;
        }
        let scale = group.world.x_axis.truncate().length().max(f32::EPSILON);
        // The group's up, as it lies on the screen after the map turned to the car's heading.
        let up = group.world.transform_vector3(Vec3::NEG_Y);
        let length = up.truncate().length().max(f32::EPSILON);
        Some(Self {
            centre: (backing.world.w_axis.x, backing.world.w_axis.y),
            radius: MAP_RADIUS * scale,
            north: (up.x / length, up.y / length),
            depth: group.z,
            opacity: f32::from(group.world_colour[3]) / 255.0,
        })
    }
}

/// The heat as the HUD shows it: a whole multiplier, 1 to 10.
pub(super) fn multiplier(heat: f32) -> u32 {
    heat.floor().clamp(1.0, HEAT_SEGMENTS as f32) as u32
}

/// How many of the ten heat segments are lit: the whole heat levels reached, 0 before the first.
pub(super) fn lit_segments(heat: f32) -> u32 {
    heat.floor().clamp(0.0, HEAT_SEGMENTS as f32) as u32
}

fn parts(
    frame: &FrameCell,
    feed: &DriveFeedCell,
    tree: &UiTree,
    window: &Window,
    cx: &LayoutCx<'_>,
) -> Vec<Box<dyn Component>> {
    let feed = feed.snapshot();
    let colour = |key: &str| cx.skin.colour(key).unwrap_or([255; 4]);
    let (x, y) = window.centre;
    let r = window.radius;
    let z = window.depth - ABOVE_MAP;
    let mut parts: Vec<Box<dyn Component>> = Vec::new();

    // The dark disc the map lies on, the map, darkened, and the arrow in the accent colour.
    let disc = Rect::new(x, y, 2.0 * r, 2.0 * r);
    parts.push(Box::new(Rounded::at_depth(keys::MINIMAP_DISC, disc, r, window.depth + BEHIND_MAP, cx)));
    // A race's map has its route baked in as a white band, which the darkening tint would turn grey: it is shown as it is.
    let map_tint = match feed.route_map {
        true => [255; 4],
        false => colour(keys::MINIMAP_MAP),
    };
    parts.push(Box::new(Tinted::new(FengSubtree::hud(frame, roots::minimap_tiles()), map_tint)));
    parts.push(Box::new(Tinted::recoloured(
        FengSubtree::hud(frame, roots::minimap_arrow()),
        colour(keys::UNIT_HIGHLIGHT),
    )));

    // The bezel.
    let ring = |key: &str, radius: f32, width: f32, from: f32, to: f32| -> Box<dyn Component> {
        Box::new(Ring::at_depth(key, (x, y), radius, width, (from, to), z, cx))
    };
    parts.push(ring(keys::MINIMAP_BEZEL, r + BEZEL_WIDTH / 2.0, BEZEL_WIDTH, 0.0, 360.0));
    parts.push(ring(keys::UNIT_HIGHLIGHT, r - HAIRLINE_WIDTH / 2.0, HAIRLINE_WIDTH, 0.0, 360.0));

    // The heat ring.
    let lit = lit_segments(feed.heat);
    let step = 360.0 / HEAT_SEGMENTS as f32;
    for k in 0..HEAT_SEGMENTS {
        let key = if k < lit { keys::MINIMAP_HEAT } else { keys::MINIMAP_HEAT_DIM };
        let from = step * k as f32 + HEAT_GAP / 2.0;
        parts.push(ring(key, r + HEAT_OFFSET, HEAT_WIDTH, from, from + step - HEAT_GAP));
    }

    // North on the bezel.
    let edge = r + BEZEL_WIDTH / 2.0;
    let north = (x + window.north.0 * edge, y + window.north.1 * edge);
    let badge = Rect::new(north.0, north.1, NORTH_BADGE, NORTH_BADGE);
    parts.push(Box::new(Rounded::at_depth(keys::MINIMAP_BEZEL, badge, NORTH_BADGE / 2.0, z - 10.0, cx)));
    let mut n = Text::new(keys::FONT_MENU, "N")
        .with_colour_key(keys::UNIT_HIGHLIGHT)
        .with_scale(NORTH_SCALE)
        .with_justification(justify::H_CENTRE | justify::V_CENTRE)
        .with_z(z - 20.0);
    n.layout(cx, badge);
    parts.push(Box::new(n));

    // The flame and the multiplier.
    let angle = FLAME_ANGLE.to_radians();
    let reach = r + FLAME_OFFSET;
    let flame = (x + reach * angle.sin(), y - reach * angle.cos());
    if let Some(texture) = flame_texture(tree) {
        let mut image = Image::raw(texture).with_tint(colour(keys::UNIT_HIGHLIGHT)).with_z(z - 10.0);
        image.layout(cx, Rect::new(flame.0, flame.1, FLAME_SIZE, FLAME_SIZE));
        parts.push(Box::new(image));
    }
    let mut label = Text::new(keys::FONT_MENU_TITLE, format!("x{}", multiplier(feed.heat)))
        .with_colour_key(keys::UNIT_HIGHLIGHT)
        .with_scale(HEAT_TEXT_SCALE)
        .with_justification(justify::V_CENTRE)
        .with_z(z - 10.0);
    let left = flame.0 + FLAME_SIZE / 2.0 + 3.0;
    label.layout(cx, Rect::new(left + HEAT_TEXT_WIDTH / 2.0, flame.1 + 1.0, HEAT_TEXT_WIDTH, 16.0));
    parts.push(Box::new(label));
    parts
}

/// The flame's texture: the one the package's own heat icon draws.
fn flame_texture(tree: &UiTree) -> Option<u32> {
    let node = tree.nodes.iter().find(|node| node.name_hash == FLAME_OBJECT)?;
    match node.kind {
        NodeKind::Image { texture, .. } => Some(texture),
        _ => None,
    }
}

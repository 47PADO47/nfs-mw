//! The objects and the layout of the widget menus: the objects of one row, and where the rows go.

use blackbox_feng::ObjectRef;
use glam::Vec2;

pub(super) fn hide_short_scrollbar(cx: &mut super::logic::Cx, rows: usize, window: usize) {
    if cx.name.eq_ignore_ascii_case(super::ids::screen::OPTIONS) && rows <= window {
        cx.hide(0xA808_E057, true); // Main options scrollbar parent.
    }
}

/// The objects of one row.
pub(super) struct RowObjects {
    pub(super) name: ObjectRef,
    pub(super) guid: u32,
    pub(super) data: Option<ObjectRef>,
    pub(super) left: Option<ObjectRef>,
    pub(super) right: Option<ObjectRef>,
    pub(super) base: Option<ObjectRef>,
    pub(super) fill: Option<ObjectRef>,
}

/// Where rows go, from the package's two master objects (the title column and the data column).
pub(super) struct Layout {
    /// Top-left of the title column, and its size.
    pub(super) title: Vec2,
    pub(super) title_size: Vec2,
    /// Top-left of the data column, and its size.
    pub(super) data: Vec2,
    pub(super) data_size: Vec2,
    /// The masters the two columns are read from; their boxes are where the rows are drawn.
    pub(super) title_object: ObjectRef,
    pub(super) data_object: ObjectRef,
}

const SCROLLBAR: u32 = 0xA808_E057;
const SCROLL_ARROWS: [u32; 2] = [0x4449_69FD, 0x4449_69FE];

fn scrollbar_groups(cx: &super::logic::Cx) -> Option<[usize; 2]> {
    if !cx.name.eq_ignore_ascii_case(super::ids::screen::OPTIONS) {
        return None;
    }
    let tree = cx.rt.tree(cx.package);
    let root = tree.nodes.iter().find(|n| n.name_hash == SCROLLBAR)?.index;
    let group = |hash| tree.nodes.iter().find(|n| n.parent == Some(root) && n.name_hash == hash).map(|n| n.index);
    Some([group(SCROLL_ARROWS[0])?, group(SCROLL_ARROWS[1])?])
}

pub(super) fn show_overflow_arrows(cx: &mut super::logic::Cx, rows: usize, window: usize) {
    if rows <= window {
        return;
    }
    if let Some(groups) = scrollbar_groups(cx) {
        for index in groups {
            cx.rt.set_hidden(ObjectRef { package: cx.package, index }, false);
        }
    }
}

pub(super) fn scrollbar_arrow_at(cx: &super::logic::Cx, point: Vec2) -> Option<bool> {
    let groups = scrollbar_groups(cx)?;
    let tree = cx.rt.tree(cx.package);
    let mut candidates = Vec::new();
    for &index in &tree.draw_order {
        let node = &tree.nodes[index];
        if node.world_colour[3] == 0 {
            continue;
        }
        let mut parent = node.parent;
        while let Some(i) = parent {
            if groups.contains(&i) {
                candidates.push(ObjectRef { package: cx.package, index });
                break;
            }
            parent = tree.nodes[i].parent;
        }
    }
    let hit = cx.rt.object_at(&candidates, point)?;
    let mut parent = tree.nodes[hit.index].parent;
    while let Some(i) = parent {
        if let Some(direction) = groups.iter().position(|&group| group == i) {
            return Some(direction == 1);
        }
        parent = tree.nodes[i].parent;
    }
    None
}

/// Hide unsupported defaults and the unscoped package arrows.
pub(super) fn hide_unused(cx: &mut super::logic::Cx) {
    for hash in [0xD646_3100, 0x28B8_FD2F, 0x2C42_0D64, 0x3FD8_A341, 0xD9A2_2505].into_iter().chain(SCROLL_ARROWS) {
        cx.hide(hash, true);
    }
}

/// Fits a fill to the base and keeps the texture tiles at their authored pitch.
pub(super) fn draw_slider(cx: &mut super::logic::Cx, row: &RowObjects, value: f32) {
    let (Some(base), Some(fill)) = (row.base, row.fill) else { return };
    let (Some(b), Some(f), Some(base_size)) = (cx.rt.position(base), cx.rt.size(fill), cx.rt.size(base)) else {
        return;
    };
    let Some(def) = cx.rt.package(cx.package).and_then(|p| p.objects.get(fill.index)) else { return };
    let mut uv = def.data.uv();
    let fallback = def.data.size().x.abs() / (uv[2] - uv[0]).abs();
    let texture = def.resource.and_then(|i| cx.rt.package(cx.package)?.resources.get(i)).map(|r| r.handle);
    let tile_width = texture.and_then(|hash| cx.assets.texture_size(hash)).map_or(fallback, |size| size.0 as f32);
    let width = base_size.x.abs() * value;
    let base_tl = Vec2::new(b.x - base_size.x.abs() * 0.5, b.y - base_size.y.abs() * 0.5);
    let tl = base_tl + Vec2::new(2.0, -12.0);
    cx.rt.set_size_xy(fill, width, f.y);
    cx.rt.set_position_xy(fill, tl.x + width * 0.5, tl.y + f.y * 0.5);
    if tile_width.is_finite() && tile_width > 0.0 {
        uv[2] = uv[0] + width / tile_width;
        cx.rt.set_uv(fill, uv);
    }
}

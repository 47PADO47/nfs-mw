//! The rear-view mirror's picture under its bezel: finding the bezel's rectangle in the tree and drawing the uploaded
//! mirror texture through the glass inside it. The presenter (`super`) uploads the picture and draws the bezel on top.

use blackbox_feng::UiTree;
use glam::{Vec3, Vec4};

use super::{MIRROR_TEXTURE, MeshBuilder, vertex_colour};

/// Name of the two bezel images of the mirror (`Mirror_Backing_Radar.tga`), in `HUD_SingleRace.fng`.
pub(super) const MIRROR_BEZEL: u32 = 0x5630_E0B9;
/// The alpha the package gives those two images on their own.
const MIRROR_OWN_ALPHA: u32 = 200;
/// The glass inside the bezel, as fractions of the bezel's width and height that are left out on each side.
const MIRROR_GLASS_INSET: [f32; 2] = [0.03, 0.05];

/// The alpha of the mirror bezel: the package's alpha (200) scaled to the full
/// range, so the fade of the group reaches 255 at the end.
pub(super) fn faded_alpha(alpha: u8) -> u8 {
    (u32::from(alpha) * 255 / MIRROR_OWN_ALPHA).min(255) as u8
}

/// The rear-view picture, over the rectangle the mirror's bezel covers. The bezel is drawn after it, so the frame
/// sits on top of the picture's edges.
pub(super) fn mirror_picture(tree: &UiTree, builder: &mut MeshBuilder, scale: f32, origin: [f32; 2]) {
    let mut lo = [f32::MAX; 2];
    let mut hi = [f32::MIN; 2];
    let mut bezel_alpha = None;
    for node in tree.draw_order.iter().map(|&i| &tree.nodes[i]).filter(|n| n.name_hash == MIRROR_BEZEL) {
        bezel_alpha.get_or_insert(node.world_colour[3]);
        for (x, y) in [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
            let p = (node.world * Vec4::new(x, y, 0.0, 1.0)).truncate();
            lo = [lo[0].min(p.x), lo[1].min(p.y)];
            hi = [hi[0].max(p.x), hi[1].max(p.y)];
        }
    }
    if lo[0] > hi[0] {
        return;
    }
    // The bezel is a frame around the glass: the picture covers the glass, not the frame. Insets are fractions of the
    // bezel's size, measured on the reference screenshot.
    let (w, h) = (hi[0] - lo[0], hi[1] - lo[1]);
    lo = [lo[0] + w * MIRROR_GLASS_INSET[0], lo[1] + h * MIRROR_GLASS_INSET[1]];
    hi = [hi[0] - w * MIRROR_GLASS_INSET[0], hi[1] - h * MIRROR_GLASS_INSET[1]];
    let corners = [
        Vec3::new(lo[0], lo[1], 0.0),
        Vec3::new(hi[0], lo[1], 0.0),
        Vec3::new(hi[0], hi[1], 0.0),
        Vec3::new(lo[0], hi[1], 0.0),
    ];
    // The picture fades with the bezel: it takes the bezel's alpha, scaled to the full range.
    let alpha = faded_alpha(bezel_alpha.unwrap_or(0));
    builder.quad(
        MIRROR_TEXTURE,
        corners,
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        vertex_colour([255, 255, 255, alpha], false),
        scale,
        origin,
    );
}

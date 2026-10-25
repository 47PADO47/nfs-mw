//! The mirror picture is clipped to the installed bezel's transparent glass, not its padded texture rectangle.
use super::{MIRROR_TEXTURE, MeshBuilder, vertex_colour};
use crate::ui::{UiAssets, assets::Image};
use blackbox_feng::{NodeKind, UiTree};
use glam::{Vec3, Vec4};
use std::collections::HashMap;
pub(super) const MIRROR_BEZEL: u32 = 0x5630_E0B9;
const MIRROR_OWN_ALPHA: u32 = 200;
pub(super) fn faded_alpha(alpha: u8) -> u8 {
    (u32::from(alpha) * 255 / MIRROR_OWN_ALPHA).min(255) as u8
}
#[derive(Default)]
pub(super) struct GlassCache(HashMap<u32, Vec<[f32; 4]>>);
// A half-bezel is open at its inner edge. Its longest transparent vertical run is the glass;
// the small holes below it belong to the detector and must never show the camera picture.
fn strips(image: &Image) -> Vec<[f32; 4]> {
    let (w, h) = (image.width as usize, image.height as usize);
    if w == 0 || h == 0 || image.rgba.len() != w * h * 4 {
        return Vec::new();
    }
    let alpha = |x: usize, y: usize| image.rgba[(y * w + x) * 4 + 3];
    let mut run = 0;
    let mut best = (0, 0);
    for y in 0..=h {
        if y < h && alpha(0, y) < 255 {
            run += 1;
            continue;
        }
        if run > best.1 {
            best = (y - run, run);
        }
        run = 0;
    }
    if best.1 == 0 {
        return Vec::new();
    }
    let mut edges: Vec<Option<f32>> = vec![None; h];
    let mut visited = vec![false; w * h];
    let mut pending = vec![(best.0 + best.1 / 2) * w];
    // Follow the entire glass component: compressed antialiasing can extend around an
    // opaque pixel or into a row whose centreline is already inside the black frame.
    while let Some(i) = pending.pop() {
        let (x, y) = (i % w, i / w);
        if visited[i] || alpha(x, y) == 255 {
            continue;
        }
        visited[i] = true;
        let edge = (x as f32 + 1.5).min(w as f32) / w as f32;
        edges[y] = Some(edges[y].map_or(edge, |old| old.max(edge)));
        if x > 0 {
            pending.push(i - 1);
        }
        if x + 1 < w {
            pending.push(i + 1);
        }
        if y > 0 {
            pending.push(i - w);
        }
        if y + 1 < h {
            pending.push(i + w);
        }
    }
    let mut out = Vec::new();
    // Bilinear sampling mixes adjacent texel rows between their centres. Cover that entire
    // footprint underneath the opaque frame, including its partially transparent inner fringe.
    // Disjoint slabs avoid double blending the picture while the mirror fades in.
    for y in 0..=h {
        let above = y.checked_sub(1).and_then(|i| edges.get(i)).copied().flatten();
        let below = edges.get(y).copied().flatten();
        let Some(edge) = above.into_iter().chain(below).max_by(f32::total_cmp) else { continue };
        out.push([0.0, ((y as f32 - 0.5) / h as f32).max(0.0), edge, ((y as f32 + 0.5) / h as f32).min(1.0)]);
    }
    out
}
pub(super) fn mirror_picture(
    tree: &UiTree,
    builder: &mut MeshBuilder,
    assets: &UiAssets,
    cache: &mut GlassCache,
    scale: f32,
    origin: [f32; 2],
) {
    let mut quads = Vec::new();
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    let mut alpha = 0;
    for node in tree.draw_order.iter().map(|&i| &tree.nodes[i]).filter(|n| n.name_hash == MIRROR_BEZEL) {
        let NodeKind::Image { texture, uv, .. } = &node.kind else { continue };
        let rows =
            cache.0.entry(*texture).or_insert_with(|| assets.image(*texture).map_or_else(Vec::new, |i| strips(&i)));
        alpha = alpha.max(node.world_colour[3]);
        for &[u0, v0, u1, v1] in rows.iter() {
            let map = |u: f32, v: f32| {
                let x = (u - uv[0]) / (uv[2] - uv[0]);
                let y = (v - uv[1]) / (uv[3] - uv[1]);
                (node.world * Vec4::new(x - 0.5, y - 0.5, 0.0, 1.0)).truncate()
            };
            if (uv[2] - uv[0]).abs() < 1e-6 || (uv[3] - uv[1]).abs() < 1e-6 {
                continue;
            }
            let corners = [map(u0, v0), map(u1, v0), map(u1, v1), map(u0, v1)];
            for &p in &corners {
                lo = lo.min(p);
                hi = hi.max(p);
            }
            quads.push(corners);
        }
    }
    let size = hi - lo;
    if quads.is_empty() || size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    let colour = vertex_colour([255, 255, 255, faded_alpha(alpha)], false);
    for corners in quads {
        let uvs = corners.map(|p| [(p.x - lo.x) / size.x, (p.y - lo.y) / size.y]);
        builder.quad(MIRROR_TEXTURE, corners, uvs, colour, scale, origin);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detector_holes_and_transparent_padding_do_not_extend_the_glass() {
        let mut image = Image { width: 16, height: 16, rgba: vec![0; 16 * 16 * 4], blend: 1 };
        for y in 2..=12 {
            for x in 0..16 {
                if y == 2 || y == 10 || x >= 14 || y == 12 {
                    image.rgba[(y * 16 + x) * 4 + 3] = 255;
                }
            }
        }
        let rows = strips(&image);
        assert_eq!(rows.len(), 8);
        assert!(rows.iter().all(|r| r[1] >= 2.5 / 16.0 && r[3] <= 10.5 / 16.0 && r[2] < 1.0));
    }

    #[test]
    fn filtered_inner_fringe_has_picture_coverage_without_overlapping_fade_strips() {
        let mut image = Image { width: 16, height: 16, rgba: vec![255; 16 * 16 * 4], blend: 1 };
        for y in 3..=11 {
            for x in 0..14 {
                let alpha = match (x, y) {
                    (_, 3) => 180,
                    (_, 11) => 220,
                    (13, _) => 200,
                    _ => 0,
                };
                image.rgba[(y * 16 + x) * 4 + 3] = alpha;
            }
        }
        // A detector hole is separate from the glass.
        image.rgba[(14 * 16) * 4 + 3] = 0;
        let rows = strips(&image);
        assert_eq!(rows.first().unwrap()[1], 2.5 / 16.0);
        assert_eq!(rows.last().unwrap()[3], 12.5 / 16.0);
        assert!(rows.iter().all(|r| r[2] == 14.5 / 16.0));
        assert!(rows.windows(2).all(|p| p[0][3] == p[1][1]), "slabs meet without overlap or cracks");
        for y in 21..100 {
            let v = y as f32 / 128.0;
            let count = rows.iter().filter(|r| v >= r[1] && v < r[3]).count();
            assert_eq!(count, 1, "every filtered glass row has exactly one picture layer");
        }
        // A curved corner's fringe may continue below the opening at the inner edge.
        image.rgba[(12 * 16 + 12) * 4 + 3] = 180;
        let curved = strips(&image);
        assert_eq!(curved.last().unwrap()[3], 13.5 / 16.0);
        assert!(curved.iter().all(|r| r[3] < 14.0 / 16.0), "the separate detector hole stays excluded");
    }
}

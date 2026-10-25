//! The game's own map picture on the overlay: the minimap tiles of the install, cut into square blocks that turn and
//! scale with the view, and a faint dark wash under them.
//!
//! **Transparency.** The tiles are additive textures (`docs/formats/minimap.md`): black adds nothing, so only the roads
//! and the water show over the world, and the tint scales how bright they are. The overlay fades toward its edge by
//! scaling the tint of each cell by the edge's fade, so no frame or mask is needed.
//!
//! **The cells.** The picture is [`BLOCKS`] blocks wide per tile. A block well inside the ellipse is one quad; a block
//! the soft edge crosses is cut into finer cells, each tinted by the fade at its middle, so the edge is smooth to a
//! cell's width. A block outside is not built. Every cell is an upright square of the picture, turned with the view.

use blackbox_ui::{Component, DrawList, Image, LayoutCx, Rect, Rounded};
use glam::Vec2;

use super::draw::Stage;
use crate::hud::MapPicture;
use crate::ui::theme::custom::tokens::{fade, faded_skin};

/// Blocks per tile side. A tile is 832 m in the city map, so a block is 104 m.
pub const BLOCKS: i32 = 8;
/// A cell near the edge is about this wide on the canvas, in HUD units at scale 1; a block is cut into at most
/// [`MAX_CUT`] cells a side.
const CELL: f32 = 14.0;
const MAX_CUT: u32 = 12;
/// The wash's alpha behind the picture, and the strips it is built from.
pub const WASH_ALPHA: f32 = 0.16;
const WASH_STRIPS: u32 = 24;

/// The picture's cells as one component, so a frame boxes one part, not a thousand.
struct Cells(Vec<Image>);

impl Component for Cells {
    /// Each cell is laid out where it is made.
    fn layout(&mut self, _cx: &LayoutCx<'_>, _bounds: Rect) {}

    fn build(&self, out: &mut DrawList) {
        out.reserve(self.0.len());
        for cell in &self.0 {
            cell.build(out);
        }
    }
}

/// Pushes the picture onto `out`: the cells in `tint` (its alpha is the picture's opacity) at depth `z`. The ellipse and
/// its soft edge are the stage's.
pub fn picture_parts(
    picture: &MapPicture,
    stage: &Stage<'_>,
    (tint, z): ([u8; 4], f32),
    cx: &LayoutCx<'_>,
    out: &mut Vec<Box<dyn Component>>,
) {
    let calibration = picture.calibration();
    let view = stage.view;
    let across = BLOCKS * picture.tiles_per_side();
    let block_m = calibration.width / across as f32;
    let block_px = block_m * view.scale;
    // The blocks under the circle that holds the ellipse, as columns and rows from the picture's top left.
    let reach = stage.radii.max_element() / view.scale;
    let column = |x: f32| ((x - calibration.origin[0]) / block_m).floor() as i32;
    let row = |y: f32| ((calibration.origin[1] + calibration.width - y) / block_m).floor() as i32;
    let columns = (column(view.car[0] - reach).max(0), column(view.car[0] + reach).min(across - 1));
    let rows = (row(view.car[1] + reach).max(0), row(view.car[1] - reach).min(across - 1));
    // The cells a block is cut into, a side, while the soft edge crosses it.
    let cut = ((block_px / (CELL * stage.scale)).ceil() as u32).clamp(1, MAX_CUT);
    let half = block_px * std::f32::consts::FRAC_1_SQRT_2;
    let mut cells = Vec::new();
    for r in rows.0..=rows.1 {
        for c in columns.0..=columns.1 {
            let Some(key) = picture.tile(c / BLOCKS, r / BLOCKS) else { continue };
            let uv = block_uv(c % BLOCKS, r % BLOCKS);
            let middle = [
                calibration.origin[0] + (c as f32 + 0.5) * block_m,
                calibration.origin[1] + calibration.width - (r as f32 + 0.5) * block_m,
            ];
            let at = view.to_screen(middle);
            let depth = stage.clip.edge_distance(at);
            if depth < -half {
                continue;
            }
            if depth >= stage.band + half {
                cells.push(cell(key, uv, (at, block_px), tint, (view.turn, z), cx));
                continue;
            }
            for (cut_at, cut_uv, size) in cut_block(view.turn, (at, block_px), uv, cut) {
                let alpha = stage.clip.fade(cut_at, stage.band);
                if alpha <= 0.0 {
                    continue;
                }
                cells.push(cell(key, cut_uv, (cut_at, size), fade(tint, alpha), (view.turn, z), cx));
            }
        }
    }
    if !cells.is_empty() {
        out.push(Box::new(Cells(cells)));
    }
}

/// The part of a tile one block shows, as a UV rectangle.
fn block_uv(column: i32, row: i32) -> [f32; 4] {
    let side = BLOCKS as f32;
    [column as f32 / side, row as f32 / side, (column + 1) as f32 / side, (row + 1) as f32 / side]
}

/// One upright square of the picture centred at `at`, `size` across, turned with the view's `turn` (counter-clockwise on
/// the screen; the image turns clockwise).
fn cell(
    texture: u32,
    uv: [f32; 4],
    (at, size): (Vec2, f32),
    tint: [u8; 4],
    (turn, z): (f32, f32),
    cx: &LayoutCx<'_>,
) -> Image {
    let mut image = Image::raw(texture).with_uv(uv).with_tint(tint).with_z(z).with_rotation(-turn);
    image.layout(cx, Rect::new(at.x, at.y, size, size));
    image
}

/// The `cut` x `cut` cells of a block centred at `at`, `size` across, showing the UV rectangle `uv`: each one's canvas
/// centre, UV rectangle and size.
fn cut_block(turn: f32, (at, size): (Vec2, f32), uv: [f32; 4], cut: u32) -> Vec<(Vec2, [f32; 4], f32)> {
    let step = size / cut as f32;
    let (sin, cos) = turn.sin_cos();
    let along = |from: f32, to: f32, k: u32| from + (to - from) * k as f32 / cut as f32;
    let mut out = Vec::with_capacity((cut * cut) as usize);
    for j in 0..cut {
        for i in 0..cut {
            // The cell's offset from the block's middle on an upright picture, then turned: canvas y points down, so
            // the view's counter-clockwise turn is the matrix below.
            let offset = Vec2::new((i as f32 + 0.5) * step - size / 2.0, (j as f32 + 0.5) * step - size / 2.0);
            let turned = Vec2::new(offset.x * cos + offset.y * sin, -offset.x * sin + offset.y * cos);
            let rectangle = [
                along(uv[0], uv[2], i),
                along(uv[1], uv[3], j),
                along(uv[0], uv[2], i + 1),
                along(uv[1], uv[3], j + 1),
            ];
            out.push((at + turned, rectangle, step));
        }
    }
    out
}

/// The dark wash behind the picture, `alpha` strong at depth `z` in the skin colour `key`: strips across the ellipse,
/// each as wide as the ellipse is at its height and as strong as the soft edge lets it be.
pub fn wash(
    stage: &Stage<'_>,
    key: &'static str,
    (alpha, z): (f32, f32),
    cx: &LayoutCx<'_>,
) -> Vec<Box<dyn Component>> {
    let (centre, radii) = (stage.view.centre, stage.radii);
    let height = 2.0 * radii.y / WASH_STRIPS as f32;
    (0..WASH_STRIPS)
        .filter_map(|k| {
            let y = centre.y - radii.y + (k as f32 + 0.5) * height;
            let t = (y - centre.y) / radii.y;
            let width = 2.0 * radii.x * (1.0 - t * t).max(0.0).sqrt();
            let strength = alpha * stage.clip.fade(Vec2::new(centre.x, y), stage.band);
            if width <= 0.0 || strength <= 0.0 {
                return None;
            }
            let skin = faded_skin(cx.skin, &[key], &[], strength);
            let strip = Rect::new(centre.x, y, width, height);
            let part = Rounded::at_depth(key, strip, 0.0, z, &LayoutCx::new(&skin, cx.time_s));
            Some(Box::new(part) as Box<dyn Component>)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_shows_an_eighth_of_a_tile_each_way() {
        assert_eq!(block_uv(0, 0), [0.0, 0.0, 0.125, 0.125]);
        assert_eq!(block_uv(7, 7), [0.875, 0.875, 1.0, 1.0]);
    }

    #[test]
    fn a_cut_block_tiles_its_uv_rectangle() {
        let cells = cut_block(0.0, (Vec2::new(100.0, 50.0), 30.0), [0.0, 0.5, 0.5, 1.0], 3);
        assert_eq!(cells.len(), 9);
        assert!((cells[0].1[2] - 0.5 / 3.0).abs() < 1e-6 && (cells[0].1[3] - (0.5 + 0.5 / 3.0)).abs() < 1e-6);
        assert!((cells[8].1[2] - 0.5).abs() < 1e-6 && (cells[8].1[3] - 1.0).abs() < 1e-6);
        // Upright, the first cell is the top left of the block and the last the bottom right.
        assert_eq!((cells[0].0, cells[8].0), (Vec2::new(90.0, 40.0), Vec2::new(110.0, 60.0)));
        assert!(cells.iter().all(|cell| cell.2 == 10.0));
    }

    #[test]
    fn a_turn_swings_the_cells_about_the_block_middle() {
        // A quarter turn counter-clockwise on the screen takes the block's top right to its top left.
        let cells = cut_block(std::f32::consts::FRAC_PI_2, (Vec2::ZERO, 20.0), [0.0, 0.0, 1.0, 1.0], 2);
        let top_right = cells[1].0;
        assert!((top_right.x + 5.0).abs() < 1e-4 && (top_right.y + 5.0).abs() < 1e-4, "{top_right:?}");
    }
}

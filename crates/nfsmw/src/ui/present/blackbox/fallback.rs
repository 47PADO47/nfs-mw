//! FEng text whose font lacks some of its characters (a language the stock fonts do not cover): each line is drawn in the
//! language's fallback fonts, at the FEng font's line height, where the FEng layout would put it
//! (`docs/specs/localization.md` section 5). Word wrap is not applied: line breaks are.

use blackbox_feng::font::TextStyle;
use blackbox_feng::{Font, UiNode};
use blackbox_ui::justify;
use glam::Vec4;

use super::{BlackboxPresenter, MeshBuilder, send, vertex_colour};
use crate::gui::UiOutput;
use crate::ui::raster_text;

impl BlackboxPresenter {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn fallback_text(
        &mut self,
        node: &UiNode,
        text: &str,
        font: &Font,
        style: TextStyle,
        font_scale: f32,
        out: &mut UiOutput,
        builder: &mut MeshBuilder,
        scale: f32,
        origin: [f32; 2],
    ) {
        let key = raster_text::fallback_key();
        let lines: Vec<&str> = text.split(['\n', '^']).collect();
        let line_height = font.height() * font_scale;
        let step = line_height + style.leading as f32 * font_scale;
        let total = step * (lines.len() as f32 - 1.0) + line_height;
        let top = if style.justification & justify::V_CENTRE != 0 {
            -total * 0.5
        } else if style.justification & justify::V_BOTTOM != 0 {
            -total
        } else {
            0.0
        };
        let colour = vertex_colour(node.world_colour, false);
        for (row, line) in lines.iter().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let (id, aspect) = match self.raster.get(line, key) {
                Some(entry) => entry,
                None => {
                    let Some(image) = raster_text::render_fallback(line) else { continue };
                    let aspect = image.width as f32 / image.height as f32;
                    let id = self.raster.insert(line, key, aspect, &mut out.freed);
                    send(id, image, out);
                    (id, aspect)
                }
            };
            let mut width = line_height * aspect;
            // A line wider than the maximum width is squeezed, as the FEng path squeezes it.
            if style.max_width != 0 && width > style.max_width as f32 {
                width = style.max_width as f32;
            }
            let left = if style.justification & justify::H_CENTRE != 0 {
                -width * 0.5
            } else if style.justification & justify::H_RIGHT != 0 {
                -width
            } else {
                0.0
            };
            let y0 = top + step * row as f32;
            let p = |x: f32, y: f32| (node.world * Vec4::new(x, y, 0.0, 1.0)).truncate();
            let corners = [p(left, y0), p(left + width, y0), p(left + width, y0 + line_height), p(left, y0 + line_height)];
            let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
            builder.quad(id, corners, uvs, colour, scale, origin);
        }
    }
}

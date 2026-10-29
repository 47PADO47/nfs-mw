//! Native seven-line tuning help panel, with the installed font's line widths.

use crate::frontend::logic::Cx;
use crate::garage::{Model, menu_rows::Row};
use blackbox_feng::font::TextStyle;
use nfsmw_data::car::physics::tuning::TuningCategory;

#[derive(Default)]
pub(super) struct Help {
    pub open: bool,
    lines: Vec<String>,
    first: usize,
}

impl Help {
    pub fn toggle(&mut self, cx: &mut Cx, model: &Model, picked: usize) {
        self.open = !self.open;
        cx.rt.post_to_package(
            cx.package,
            match self.open {
                true => 0x89d3_32a9,
                false => 0x950a_d1c2,
            },
        );
        if !self.open {
            return;
        }
        self.first = 0;
        let turbo = model.selected().is_none_or(|car| car.performance_details.turbo);
        let active = Row::Tuning(TuningCategory::ALL[picked]).enabled(model);
        let title = TITLE[picked];
        let mut text = match active {
            true => ACTIVE[picked],
            false => INACTIVE[picked],
        };
        if picked == 6 && !turbo {
            text = match active {
                true => 0xd70f_7468,
                false => 0xe3f5_77af,
            };
        }
        let title = match picked == 6 && !turbo {
            true => 0xe3a9_32e0,
            false => title,
        };
        for hash in [0x0db2_1da1, 0xc849_2960] {
            if let Some(object) = cx.object(hash) {
                cx.rt.set_label(object, title);
            }
        }
        let value = cx.rt.label_text(text).unwrap_or_default();
        self.lines = wrap(cx, &value);
        self.refresh(cx);
    }

    pub fn scroll(&mut self, cx: &mut Cx, by: isize) {
        self.first = self.first.saturating_add_signed(by).min(self.lines.len().saturating_sub(7));
        self.refresh(cx);
    }

    pub fn refresh(&self, cx: &mut Cx) {
        for line in 0..7 {
            cx.text_object(0x46be_6be4 + line as u32, self.lines.get(self.first + line).map_or("", String::as_str));
        }
    }
}

fn wrap(cx: &Cx, text: &str) -> Vec<String> {
    let info = cx.object(0x46be_6be4).and_then(|object| cx.rt.string_info(object));
    let Some(info) = info else { return text.lines().map(str::to_owned).collect() };
    let Some(font) = cx.assets.font(info.font) else { return text.lines().map(str::to_owned).collect() };
    let size = cx.assets.texture_size(cx.assets.resolve(font.texture_hash)).unwrap_or((256, 256));
    let mut lines = Vec::new();
    for paragraph in text.split(['^', '\n']) {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let next = match line.is_empty() {
                true => word.to_owned(),
                false => format!("{line} {word}"),
            };
            // Native wrapping happens before the object's animated scale is applied.
            let width = font.layout_scaled(&next, TextStyle::default(), size, cx.assets.font_scale(info.font)).width;
            if line.is_empty() || width <= info.max_width.max(1) as f32 {
                line = next;
                continue;
            }
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        }
        lines.push(line);
    }
    lines
}

const TITLE: [u32; 7] = [0xc56c_5b36, 0xfcee_be1a, 0x2ee2_a74d, 0xcf62_15d1, 0x7196_acb4, 0x934f_cfa9, 0x8694_5521];
const ACTIVE: [u32; 7] = [0x4a1f_18be, 0x81a1_7ba2, 0x4738_62d5, 0xd6c2_4659, 0x64fc_ee3c, 0xb65c_fc31, 0xb5dc_bfa9];
const INACTIVE: [u32; 7] = [0x221d_7e85, 0x18c1_2069, 0xc213_a6dc, 0xb6d0_2c60, 0xc6a9_9483, 0xb812_4038, 0xb7d6_f7b0];

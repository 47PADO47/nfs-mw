//! Original seven sliders and three tuning profiles, backed by native selections.

use super::super::{GarageUi, Page, Request};
use crate::frontend::logic::{Cx, Footer, ScreenLogic};
use crate::garage::{MenuEvent, Selection, menu_rows::Row};
use blackbox_feng::{ObjectRef, ids::*};
use glam::Vec2;
use nfsmw_data::car::physics::tuning::TuningCategory;

mod help;

pub(super) struct Tuning {
    handle: GarageUi,
    entry: Selection,
    picked: usize,
    names: Vec<ObjectRef>,
    done: bool,
    help: help::Help,
    failure: u64,
}

impl Tuning {
    pub fn new(handle: GarageUi) -> Self {
        Self {
            entry: handle.model().draft,
            failure: handle.failure(),
            handle,
            picked: 0,
            names: Vec::new(),
            done: false,
            help: help::Help::default(),
        }
    }

    fn refresh(&self, cx: &mut Cx) {
        let model = self.handle.model();
        for hash in [0xf200_d4d9, 0x7c90_b298] {
            if let Some(object) = cx.object(hash) {
                cx.rt.set_label(object, 0x4023_0063 + model.draft.tuning.active.min(2) as u32);
            }
        }
        for (index, category) in TuningCategory::ALL.into_iter().enumerate() {
            let row = Row::Tuning(category);
            let value = row.options(&model).1 as f32 / 10.0;
            if let Some(object) = cx.named(&format!("OPTION_DATA_{}", index + 1)) {
                cx.rt.set_text(object, format!("{:.1}", value * 10.0));
            }
            if let Some(object) = self.names.get(index).copied() {
                let alpha = match row.enabled(&model) {
                    true => 255,
                    false => 75,
                };
                cx.rt.set_alpha(object, alpha);
                if index == self.picked {
                    cx.rt.focus_button(object);
                }
            }
            for (prefix, length) in
                [("LEFT_SLIDER_", (0.5 - value).max(0.0)), ("RIGHT_SLIDER_", (value - 0.5).max(0.0))]
            {
                resize(cx, &format!("{prefix}{}", index + 1), length * 2.0);
            }
        }
    }

    fn back(&mut self) {
        if self.done {
            return;
        }
        if self.handle.model().draft.tuning != self.entry.tuning {
            self.handle.request(Request::Push(Page::TuningDiscard));
            return;
        }
        self.done = true;
        self.handle.request(Request::event(MenuEvent::Cancel));
    }

    fn adjust(&mut self, by: isize) {
        let row = Row::Tuning(TuningCategory::ALL[self.picked]);
        let selected = row.options(&self.handle.model()).1;
        let Some(at) = selected.checked_add_signed(by).filter(|at| *at <= 10) else { return };
        self.handle.preview(row, at);
    }
}

impl ScreenLogic for Tuning {
    fn covers_screens(&self) -> bool {
        true
    }
    fn start(&mut self, cx: &mut Cx) {
        self.names = (1..=7).filter_map(|index| cx.named(&format!("OPTION_NAME_{index}"))).collect();
        self.refresh(cx);
    }
    fn resume(&mut self, cx: &mut Cx) {
        self.done = false;
        self.refresh(cx);
    }
    fn message(&mut self, cx: &mut Cx, message: u32) {
        if self.done {
            return;
        }
        if self.help.open {
            match message {
                PAD_UP => self.help.scroll(cx, -1),
                PAD_DOWN => self.help.scroll(cx, 1),
                PAD_BACK | PAD_ACCEPT => self.help.toggle(cx, &self.handle.model(), self.picked),
                _ => {}
            }
            return;
        }
        match message {
            PAD_UP => self.picked = self.picked.saturating_sub(1),
            PAD_DOWN => self.picked = (self.picked + 1).min(6),
            PAD_LEFT => self.adjust(-1),
            PAD_RIGHT => self.adjust(1),
            PAD_LTRIGGER | PAD_RTRIGGER => {
                let current = self.handle.model().draft.tuning.active as usize;
                let by = match message {
                    PAD_LTRIGGER => 2,
                    _ => 1,
                };
                self.handle.preview(Row::Profile, (current + by) % 3);
            }
            PAD_ACCEPT | BUTTON_PRESSED => {
                self.done = true;
                self.handle.request(Request::SaveTuning { resume: false });
            }
            PAD_START => {
                self.done = true;
                self.handle.request(Request::SaveTuning { resume: true });
            }
            PAD_BACK => self.back(),
            _ => {}
        }
        self.refresh(cx);
    }
    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if self.help.open {
            return true;
        }
        let Some(object) = cx.rt.object_at(&self.names, point) else { return false };
        self.picked = self.names.iter().position(|name| *name == object).unwrap_or(0);
        self.refresh(cx);
        true
    }
    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        if self.handle.recover(cx, &mut self.failure) {
            self.done = false;
        }
        self.refresh(cx);
        if self.help.open {
            self.help.refresh(cx);
        }
    }
    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        if bits & (blackbox_feng::runtime::pad::BUTTON0 << 5) != 0 {
            self.help.toggle(cx, &self.handle.model(), self.picked);
        }
    }
    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        if self.help.open {
            self.help.scroll(cx, -(steps as isize));
            return;
        }
        crate::frontend::logic::wheel_as_rows(self, cx, steps);
    }
    fn footer(&self) -> Footer {
        Footer::GarageTuning
    }
}

fn resize(cx: &mut Cx, name: &str, fraction: f32) {
    let (Some(fill), Some(base)) = (cx.named(&format!("FILLBAR_{name}")), cx.named(&format!("BASE_{name}"))) else {
        return;
    };
    let Some(package) = cx.rt.package(cx.package) else { return };
    let (Some(fill_def), Some(base_def)) = (package.objects.get(fill.index), package.objects.get(base.index)) else {
        return;
    };
    let size = fill_def.data.size();
    let base_size = base_def.data.size();
    let base_position = base_def.data.position();
    let mut uv = fill_def.data.uv();
    let texture = fill_def.resource.and_then(|index| package.resources.get(index)).map(|resource| resource.handle);
    let texture_width = texture
        .and_then(|hash| cx.assets.texture_size(hash))
        .map(|size| size.0 as f32)
        .unwrap_or_else(|| size.x.abs() / (uv[2] - uv[0]).abs());
    if !texture_width.is_finite() || texture_width <= 0.0 {
        return;
    }
    let fraction = fraction.clamp(0.0, 1.0);
    let width = base_size.x * fraction;
    let anchor = base_position.truncate() - base_size.truncate() * 0.5 + Vec2::new(2.0, -12.0);
    cx.rt.set_size_xy(fill, width, size.y);
    cx.rt.set_position_xy(fill, anchor.x + width * 0.5, anchor.y + size.y * 0.5);
    uv[2] = width / texture_width;
    cx.rt.set_uv(fill, uv);
    cx.rt.set_hidden(fill, fraction <= 0.0);
}

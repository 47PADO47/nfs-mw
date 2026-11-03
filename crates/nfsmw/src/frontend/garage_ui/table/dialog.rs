//! The game's two-answer dialog for deleting a car and discarding a session.

use super::super::{GarageUi, Page, Request};
use crate::frontend::{
    confirm,
    logic::{Cx, Footer, ScreenLogic},
};
use crate::garage::MenuEvent;
use blackbox_feng::ids::*;
use glam::Vec2;

pub(super) struct Dialog {
    handle: GarageUi,
    page: Page,
    yes: bool,
    done: bool,
    pending: Option<(bool, f32)>,
}

impl Dialog {
    pub fn new(handle: GarageUi, page: Page) -> Self {
        Self { handle, page, yes: false, done: false, pending: None }
    }

    fn refresh(&self, cx: &mut Cx) {
        cx.text_object(0x42ad_b44c, &self.page.title());
        match self.page {
            Page::Delete => {
                if let Some(object) = cx.object(confirm::MESSAGE) {
                    cx.rt.set_label(object, 0x4f68_196e);
                }
            }
            Page::InsufficientFunds => {
                if let Some(object) = cx.object(confirm::MESSAGE) {
                    cx.rt.set_label(object, 0x0a98_4a42);
                }
                if let Some(object) = cx.object(0x42ad_b44c) {
                    cx.rt.set_label(object, 0x6fd9_1524);
                }
                for hash in [0x0d4f_4069, 0xfac8_8427] {
                    if let Some(object) = cx.object(hash) {
                        cx.rt.set_texture(object, 0x6948_e2b3);
                    }
                }
            }
            Page::TuningDiscard => {
                for hash in [0xe2ee_606b, 0xe2ee_606c] {
                    if let Some(object) = cx.object(hash) {
                        cx.rt.set_label(object, 0xe9cb_802f);
                    }
                }
            }
            Page::TuningError => {
                let error = self.handle.model().message.and_then(Result::err).unwrap_or_default();
                for hash in [0xe2ee_606b, 0xe2ee_606c] {
                    cx.text_object(hash, &error);
                }
            }
            Page::MaxPerformance => {
                if let Some(object) = cx.object(confirm::MESSAGE) {
                    cx.rt.set_label(object, 0x892c_b612);
                }
            }
            Page::Checkout { backroom, quick } => {
                let key = match (backroom, quick) {
                    (true, _) => 0x0481_0898,
                    (false, true) => 0x71d9_e710,
                    _ => 0x8eba_a44b,
                };
                if let Some(object) = cx.object(confirm::MESSAGE) {
                    cx.rt.set_label(object, key);
                }
            }
            _ => cx.text_object(confirm::MESSAGE, "Discard your changes?"),
        }
        for (hash, label) in [(confirm::YES_BUTTON, confirm::YES), (confirm::NO_BUTTON, confirm::NO)] {
            if let Some(object) = cx.object(hash) {
                cx.rt.set_label(object, label);
            }
        }
        cx.hide(0xfde0_9072, true);
        if let Some(object) = cx.guid(0x000f_731f) {
            cx.rt.set_hidden(object, true);
        }
        if matches!(self.page, Page::Delete | Page::TuningDiscard | Page::TuningError) {
            if let Some(object) = cx.object(0x42ad_b44c) {
                cx.rt.set_label(object, 0x6fd9_1524);
            }
            for hash in [0x0d4f_4069, 0x0017_16a8, 0xfac8_8427] {
                if let Some(object) = cx.object(hash) {
                    cx.rt.set_texture(object, 0x6948_e2b3);
                }
            }
        }
        if self.notice() {
            cx.hide(confirm::NO_BUTTON, true);
            if let Some(object) = cx.object(confirm::YES_BUTTON) {
                cx.rt.set_label(object, confirm::CONTINUE);
            }
            if let Some(object) = cx.guid(0x000f_72c2) {
                cx.rt.set_hidden(object, true);
            }
        }
        if let Some(object) = cx.guid(match self.yes || self.notice() {
            true => 0x000f_70e8,
            false => 0x000f_72c2,
        }) {
            cx.rt.focus_button(object);
        }
    }

    fn notice(&self) -> bool {
        matches!(self.page, Page::InsufficientFunds | Page::TuningError)
    }

    fn answer(&mut self, yes: bool) {
        if self.done {
            return;
        }
        self.done = true;
        self.handle.request(Request::Pop);
        if self.notice() {
            return;
        }
        if !yes {
            return;
        }
        if self.page == Page::TuningDiscard {
            // The retained tuning page owns its native leave sequence after this popup closes.
            self.handle.0.lock().unwrap().tuning_discard = true;
            return;
        }
        if matches!(self.page, Page::Checkout { .. }) {
            self.handle.request(Request::Save);
            return;
        }
        if self.page == Page::MaxPerformance {
            let mut model = self.handle.model();
            if let Some(maximum) = model.maximum_regular {
                model.draft.performance = maximum;
                self.handle.restore(model.draft);
                self.handle.add_to_cart();
            }
            return;
        }
        if self.page == Page::Leave && self.handle.model().custom_slots_enabled {
            let mut state = self.handle.0.lock().unwrap();
            let selection = state.baseline.clone();
            state.cart = selection.clone();
            drop(state);
            self.handle.restore(selection);
            self.handle.request(Request::Pop);
            return;
        }
        let event = match self.page {
            Page::Delete => MenuEvent::DeleteSlot,
            _ => MenuEvent::Cancel,
        };
        self.handle.request(Request::event(event));
    }
}

impl ScreenLogic for Dialog {
    fn accepts_input(&self) -> bool {
        !self.done && self.pending.is_none()
    }

    fn start(&mut self, cx: &mut Cx) {
        self.refresh(cx);
    }
    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT => {
                self.yes = true;
                self.refresh(cx);
            }
            PAD_RIGHT => {
                self.yes = false;
                self.refresh(cx);
            }
            PAD_BACK => self.answer(false),
            PAD_ACCEPT | confirm::ANSWERED => self.answer(self.yes),
            _ => {}
        }
    }
    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        let objects: Vec<_> = [0x000f_70e8, 0x000f_72c2].into_iter().filter_map(|guid| cx.guid(guid)).collect();
        let Some(hit) = cx.rt.object_at(&objects, point) else { return false };
        self.answer(objects.first() == Some(&hit));
        true
    }
    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        use blackbox_feng::runtime::pad;
        // InGameDialog's buttons animate internally; this package has no game-facing answer messages.
        if !matches!(self.page, Page::TuningDiscard | Page::TuningError) || self.done || self.pending.is_some() {
            return;
        }
        if bits & pad::LEFT != 0 {
            self.yes = true;
        }
        if bits & pad::RIGHT != 0 {
            self.yes = false;
        }
        if bits & pad::BACK != 0 {
            self.answer(false);
        }
        if bits & pad::ACCEPT != 0 {
            // Its authored pressed script lasts 300 native ticks (1200 ticks per second).
            self.pending = Some((self.yes, 0.25));
        }
        self.refresh(cx);
    }
    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        if let Some((yes, time)) = self.pending {
            self.pending = Some((yes, time - dt));
            if time <= dt {
                self.pending = None;
                self.answer(yes);
            }
        }
        self.refresh(cx);
    }
    fn footer(&self) -> Footer {
        Footer::BackAccept
    }
}

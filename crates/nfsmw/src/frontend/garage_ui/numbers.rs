//! Numbers.fng edits the two digits and applies both door pairs as one item.

use super::{GarageUi, Page, Request, chrome};
use crate::frontend::logic::{Cx, Footer, ScreenLogic};
use crate::garage::{MenuEvent, Model, menu_rows::Row};
use blackbox_feng::{ObjectRef, ids::*};
use glam::Vec2;

pub(super) struct Numbers {
    handle: GarageUi,
    digits: [Option<u8>; 2],
    choices: [Vec<(u8, usize)>; 2],
    active: usize,
    objects: Vec<ObjectRef>,
}

impl Numbers {
    pub fn new(handle: GarageUi) -> Self {
        let model = handle.model();
        let choices = [0, 1].map(|digit| choices(digit, &model));
        let digits = [0, 1].map(|digit| {
            let at = Row::Number(digit).options(&model).1;
            choices[digit].iter().find(|(_, option)| *option == at).map(|(value, _)| *value)
        });
        Self { handle, digits, choices, active: 0, objects: Vec::new() }
    }

    fn refresh(&self, cx: &mut Cx) {
        self.handle.header(cx, Page::Numbers);
        for (digit, object) in self.objects.iter().copied().enumerate() {
            let text = self.digits[digit].map(|value| value.to_string()).unwrap_or_else(|| "-".into());
            cx.rt.set_text(object, text);
            chrome::focus(cx, object, digit == self.active);
            if digit == self.active {
                cx.rt.focus_button(object);
            }
        }
        cx.hide(0x7a83_55d9, self.digits.iter().any(Option::is_none));
        let state = self.handle.0.lock().unwrap();
        let row = Row::Number(0);
        super::details::refresh(cx, &state.model, row, row.options(&state.model).1, &state.baseline, &state.cart);
    }

    fn preview(&self) {
        let mut state = self.handle.0.lock().unwrap();
        for digit in 0..2 {
            let option = self.digits[digit]
                .and_then(|value| {
                    self.choices[digit].iter().find(|(candidate, _)| *candidate == value).map(|(_, option)| *option)
                })
                .unwrap_or(0);
            Row::Number(digit).set(&mut state.model, option);
        }
        let selection = state.model.draft.clone();
        state.requests.push(Request::event(MenuEvent::Preview(selection)));
    }

    fn step(&mut self, by: i8) {
        if self.digits.iter().any(Option::is_none) {
            self.digits = [Some(0); 2];
            self.preview();
            return;
        }
        self.digits[self.active] = self.digits[self.active].map(|value| (value as i8 + by).rem_euclid(10) as u8);
        self.preview();
    }
}

fn choices(digit: usize, model: &Model) -> Vec<(u8, usize)> {
    Row::Number(digit)
        .options(model)
        .0
        .into_iter()
        .enumerate()
        .skip(1)
        .filter_map(|(option, label)| {
            let value = label.chars().next_back()?.to_digit(10)? as u8;
            Some((value, option))
        })
        .collect()
}

impl ScreenLogic for Numbers {
    fn covers_screens(&self) -> bool {
        true
    }
    fn start(&mut self, cx: &mut Cx) {
        self.objects = [0x2a08_ba92, 0x1a88_dc05].into_iter().filter_map(|hash| cx.object(hash)).collect();
        self.handle.enter(Page::Numbers);
        self.refresh(cx);
    }
    fn resume(&mut self, cx: &mut Cx) {
        self.preview();
        self.handle.enter(Page::Numbers);
        self.refresh(cx);
    }
    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_LEFT | PAD_RIGHT => self.active = 1 - self.active,
            PAD_UP => self.step(1),
            PAD_DOWN => self.step(-1),
            PAD_ACCEPT | BUTTON_PRESSED => {
                if self.digits.iter().all(Option::is_some) && !self.handle.model().selection_locked {
                    self.handle.add_to_cart();
                }
                cx.rt.post_to_package(cx.package, crate::frontend::ids::INPUT_ENABLE);
            }
            PAD_BACK => {
                let cart = self.handle.0.lock().unwrap().cart.clone();
                self.handle.restore(cart);
                self.handle.request(Request::Pop);
            }
            PAD_START => self.handle.show_cart(),
            _ => {}
        }
        self.refresh(cx);
    }
    fn pad_pressed(&mut self, _cx: &mut Cx, bits: u32) {
        use blackbox_feng::runtime::pad;
        if bits & (pad::BUTTON0 << 4) != 0 {
            self.digits = [None; 2];
            self.preview();
            self.handle.add_to_cart();
        }
        if bits & pad::BUTTON0 != 0 {
            self.handle.request(Request::Push(Page::Showcase));
        }
    }
    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        let Some(hit) = cx.rt.object_at(&self.objects, point) else { return false };
        self.active = self.objects.iter().position(|object| *object == hit).unwrap_or(0);
        self.refresh(cx);
        true
    }
    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        crate::frontend::logic::wheel_as_rows(self, cx, steps);
    }
    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.refresh(cx);
    }
    fn footer(&self) -> Footer {
        Footer::GarageNumbers
    }
}

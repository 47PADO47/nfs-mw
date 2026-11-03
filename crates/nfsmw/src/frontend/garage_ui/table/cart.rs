//! The installed shopping cart's rows and purchase/remove actions.

mod names;
mod rows;

use super::super::{GarageUi, Page, Request, chrome};
use crate::frontend::logic::{Cx, Footer, ScreenLogic};
use crate::garage::menu_rows::Row;
use blackbox_feng::{ObjectRef, ids::*};
use glam::Vec2;

pub(super) struct Cart {
    handle: GarageUi,
    page: Page,
    rows: Vec<Row>,
    picked: usize,
    objects: Vec<ObjectRef>,
    submitted: bool,
    disabled: Vec<Row>,
    failure: u64,
}

impl Cart {
    pub fn new(handle: GarageUi, page: Page) -> Self {
        let failure = handle.failure();
        Self {
            handle,
            page,
            rows: Vec::new(),
            picked: 0,
            objects: Vec::new(),
            submitted: false,
            disabled: Vec::new(),
            failure,
        }
    }

    fn refresh(&mut self, cx: &mut Cx) {
        let state = self.handle.0.lock().unwrap();
        self.rows = rows::changed(&state.model, &state.baseline, &state.cart);
        self.picked = self.picked.min(self.rows.len().saturating_sub(1));
        chrome::refresh(cx, self.page, &state.model);
        super::super::finance::refresh(cx, self.page, &state.model, &state.baseline, &state.cart, state.section);
        let count = match self.page {
            Page::Cart { quick: true, .. } => 6,
            _ => 4,
        };
        let base = self.picked / count * count;
        let title = master(cx, 0xa753_c46c);
        let data = master(cx, 0xc128_b184);
        let mut costs = Vec::new();
        for row in &self.rows {
            let item = state
                .model
                .pricing
                .as_ref()
                .zip(state.model.selected())
                .filter(|_| state.model.shop && !state.model.backroom)
                .and_then(|(pricing, car)| pricing.item(*row, &state.cart, &state.baseline, car, false).ok());
            costs.push(item.map(|item| (item.cost, item.trade_in)));
        }
        for index in 0..25 {
            let visible = index < count && base + index < self.rows.len();
            let number = index + 1;
            let y = title.1 + index as f32 * title.2;
            if let Some(object) = cx.named(&format!("PART_NAME_{number}")) {
                cx.rt.set_hidden(object, !visible);
                if visible {
                    let row = self.rows[base + index];
                    cx.rt.set_text(object, names::label(row, &state.model, &state.cart));
                    cx.rt.set_position_xy(object, title.0 + 30.0, y);
                    chrome::focus(cx, object, base + index == self.picked);
                    if !rows::active(&state.model, &state.cart, row) {
                        cx.rt.set_alpha(object, 100);
                    }
                    if base + index == self.picked && cx.rt.current_button(cx.package) != Some(object) {
                        cx.rt.focus_button(object);
                    }
                }
            }
            for (prefix, x, field) in [("PRICE_", data.0 + 40.0, 0), ("TRADE_IN_", data.0 + 150.0, 1)] {
                if let Some(object) = cx.named(&format!("{prefix}{number}")) {
                    cx.rt.set_hidden(object, !visible);
                    cx.rt.set_position_xy(object, x, y);
                    let value = costs.get(base + index).copied().flatten().map(|cost| match field {
                        0 => cost.0,
                        _ => cost.1,
                    });
                    let text = value
                        .filter(|_| field == 0 || matches!(self.rows.get(base + index), Some(Row::Part(part)) if nfsmw_data::car::customization::Category::MODELS.contains(part)))
                        .map(|cost| crate::i18n::fmt::grouped(i64::from(cost)))
                        .unwrap_or_default();
                    cx.rt.set_text(object, text);
                }
            }
            if let Some(object) = cx.named(&format!("CHECK_ICON_{number}")) {
                cx.rt.set_hidden(object, !visible);
                cx.rt.set_position_xy(object, title.0, y - 10.0);
                let script =
                    match self.rows.get(base + index).is_some_and(|row| !rows::active(&state.model, &state.cart, *row))
                    {
                        true => "UNCHECKED",
                        false => "CHECKED",
                    };
                let script = blackbox_feng::fe_hash_upper(script);
                let active =
                    self.rows.get(base + index).is_some_and(|row| rows::active(&state.model, &state.cart, *row));
                cx.rt.set_texture(
                    object,
                    match active {
                        true => 0x696a_e039,
                        false => 0xe719_881c,
                    },
                );
                if cx.rt.script_of(object) != Some(script) {
                    cx.rt.run_script(object, script);
                }
            }
            if let Some(object) = cx.named(&format!("OPTION_BACKING_{number}")) {
                cx.rt.set_hidden(object, true);
            }
        }
        if state.model.message.as_ref().is_some_and(Result::is_err) {
            self.submitted = false;
        }
    }

    fn buy(&mut self) {
        if self.submitted {
            return;
        }
        let model = self.handle.model();
        if !crate::garage::can_confirm(&model) {
            if !model.backroom && model.shop_cash.zip(model.shop_total).is_some_and(|(cash, total)| total > cash) {
                self.handle.request(Request::Push(Page::InsufficientFunds));
            }
            return;
        }
        if model.draft == self.handle.0.lock().unwrap().baseline {
            self.prune();
            self.handle.request(Request::Main);
            return;
        }
        self.submitted = true;
        self.handle.request(Request::Push(Page::Checkout { backroom: model.backroom, quick: !model.shop }));
    }

    fn toggle(&mut self) {
        let Some(row) = self.rows.get(self.picked).copied() else { return };
        if matches!(row, Row::Number(_)) {
            let disabled = !self.disabled.contains(&row);
            self.disabled.retain(|candidate| !matches!(candidate, Row::Number(_)));
            if disabled {
                self.disabled.extend([Row::Number(0), Row::Number(1)]);
            }
            self.preview_checked();
            return;
        }
        if let Some(at) = self.disabled.iter().position(|candidate| *candidate == row) {
            self.disabled.remove(at);
            self.preview_checked();
            return;
        }
        self.disabled.push(row);
        self.preview_checked();
    }

    fn preview_checked(&self) {
        let mut state = self.handle.0.lock().unwrap();
        let baseline = state.baseline.clone();
        let mut selection = state.cart.clone();
        for row in &self.disabled {
            rows::restore(*row, &baseline, &mut selection);
        }
        state.model.draft = selection.clone();
        crate::garage::menu_rows::clear_missing_decal_surfaces(&mut state.model);
        let selection = state.model.draft.clone();
        state.requests.push(Request::event(crate::garage::MenuEvent::Preview(selection)));
    }

    fn prune(&mut self) {
        self.preview_checked();
        let mut state = self.handle.0.lock().unwrap();
        state.cart = state.model.draft.clone();
    }
}

impl ScreenLogic for Cart {
    fn covers_screens(&self) -> bool {
        true
    }
    fn start(&mut self, cx: &mut Cx) {
        self.objects = (1..=25).filter_map(|index| cx.named(&format!("PART_NAME_{index}"))).collect();
        self.refresh(cx);
    }
    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            PAD_UP => self.picked = self.picked.saturating_sub(1),
            PAD_DOWN => self.picked = (self.picked + 1).min(self.rows.len().saturating_sub(1)),
            PAD_BACK => {
                self.prune();
                self.handle.request(Request::Pop);
            }
            PAD_ACCEPT | BUTTON_PRESSED | PAD_START => self.buy(),
            _ => {}
        }
        self.refresh(cx);
    }
    fn pad_pressed(&mut self, _cx: &mut Cx, bits: u32) {
        use blackbox_feng::runtime::pad;
        if bits & (pad::BUTTON0 << 4) != 0 {
            self.toggle();
        }
        if bits & (pad::BUTTON0 << 5) != 0 {
            self.disabled.clone_from(&self.rows);
            self.preview_checked();
        }
    }
    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        let Some(hit) = cx.rt.object_at(&self.objects, point) else { return false };
        let Some(at) = self.objects.iter().position(|object| *object == hit) else { return false };
        let count = match self.page {
            Page::Cart { quick: true, .. } => 6,
            _ => 4,
        };
        self.picked = (self.picked / count * count + at).min(self.rows.len().saturating_sub(1));
        self.refresh(cx);
        true
    }
    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        crate::frontend::logic::wheel_as_rows(self, cx, steps);
    }
    fn resume(&mut self, cx: &mut Cx) {
        self.submitted = false;
        self.refresh(cx);
    }
    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        if self.handle.recover(cx, &mut self.failure) {
            self.submitted = false;
        }
        self.refresh(cx);
    }
    fn footer(&self) -> Footer {
        let state = self.handle.0.lock().unwrap();
        Footer::GarageCart {
            active: self.rows.get(self.picked).is_some_and(|row| rows::active(&state.model, &state.cart, *row)),
        }
    }
}

fn master(cx: &mut Cx, hash: u32) -> (f32, f32, f32) {
    let Some(object) = cx.object(hash) else { return (0.0, 0.0, 0.0) };
    cx.rt.set_hidden(object, true);
    let Some(def) = cx.rt.package(cx.package).and_then(|package| package.objects.get(object.index)) else {
        return (0.0, 0.0, 0.0);
    };
    let position = def.data.position();
    let size = def.data.size();
    (position.x - size.x * 0.5, position.y - size.y * 0.5, size.y)
}

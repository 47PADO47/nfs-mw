//! Installed icon carousels, sharing the game's icon strip and scene transactions.

use super::{GarageUi, Page, Request, choices, chrome};
use crate::frontend::{
    icon_strip::{Face, Hit, IconStrip, WHITE},
    ids,
    logic::{Cx, Footer, ScreenLogic},
};
use blackbox_feng::ids::{BUTTON_PRESSED, PAD_ACCEPT, PAD_BACK, PAD_LEFT, PAD_RIGHT, PAD_START};
use glam::Vec2;
use nfsmw_data::car::customization::Category as Part;

mod filters;
mod names;

pub(super) struct Carousel {
    handle: GarageUi,
    page: Page,
    strip: IconStrip,
    entries: Vec<choices::Item>,
    picked: usize,
    pending: Option<Request>,
    leaving: bool,
    filter_cursors: [usize; 4],
    failure: u64,
    decal_black: bool,
}

impl Carousel {
    pub fn new(handle: GarageUi, mut page: Page) -> Self {
        let model = handle.model();
        if let Page::Spoilers { backroom, .. } = page {
            let group = model
                .selected()
                .and_then(|car| car.parts.as_ref())
                .and_then(|catalog| catalog.part_groups.get(&model.draft.parts.parts[Part::Spoiler as usize]?))
                .copied()
                .filter(|group| *group < 4)
                .unwrap_or(0);
            page = Page::Spoilers { group, backroom };
        }
        let mut entries = choices::items(page, &model);
        let mut decal_black = true;
        if let Page::Choices { row: row @ crate::garage::menu_rows::Row::Decal(_, _), .. } = page {
            decal_black = choices::decal::initial_black(&model, row);
            let current = row.options(&model).1;
            let brand = choices::decal::brand(&model, row, current);
            choices::decal::filter(&model, row, decal_black, &mut entries, brand);
        }
        let strip = IconStrip::new(entries.iter().map(|item| Face::new(item.texture, item.enabled)).collect(), WHITE);
        let failure = handle.failure();
        Self {
            handle,
            page,
            strip,
            entries,
            picked: 0,
            pending: None,
            leaving: false,
            filter_cursors: [0; 4],
            failure,
            decal_black,
        }
    }

    fn refresh(&self, cx: &mut Cx) {
        self.handle.header(cx, self.page);
        if self.page == Page::Showcase {
            super::inspection::refresh(cx, &self.handle.model());
            return;
        }
        if matches!(
            self.page,
            Page::Choices { row: crate::garage::menu_rows::Row::Gauge, .. } | Page::GaugeColors { .. }
        ) {
            super::gauge_preview::refresh(cx, &self.handle.model());
        }
        if let Some(item) = self.entries.get(self.picked) {
            chrome::selection(cx, &item.label, self.picked + 1, self.entries.len());
        }
        if let Page::Spoilers { group, .. } = self.page
            && let Some(object) = cx.object(0x7800_8599)
        {
            cx.rt.set_label(object, [0x01f0_e2b2, 0x0205_b328, 0x0991_2746, 0x0e74_16fc][group as usize]);
        }
        if let Page::Rims { brand, radius, .. } = self.page {
            cx.hide(0xe678_2841, brand == 0);
            cx.text_object(0xe678_2841, &format!("{}\"", crate::i18n::fmt::grouped(i64::from(radius))));
        }
        self.filter_caption(cx);
        let state = self.handle.0.lock().unwrap();
        super::details::comparison_bars(cx, &state.model, &state.baseline);
        if let Some(row) = self.row() {
            let option = self.entries.get(self.picked).and_then(|item| item.option).unwrap_or(0);
            super::details::refresh(cx, &state.model, row, option, &state.baseline, &state.cart);
        }
        if let Some(item) = self.entries.get(self.picked) {
            names::refresh(cx, self.page, item, &state.model);
        }
    }

    fn choose(&mut self, cx: &mut Cx, picked: usize) {
        if picked >= self.entries.len() {
            return;
        }
        self.picked = picked;
        cx.memory.insert(format!("garage:{:?}", self.page), picked);
        self.strip.select(cx, self.strip.first() + picked);
        cx.rt.set_focus(cx.package, self.strip.focus_guid());
        if let Some(row) = self.row() {
            let option = self.entries.get(picked).and_then(|item| item.option).unwrap_or(picked);
            self.handle.preview(row, option);
        }
        self.refresh(cx);
    }

    fn shift(&mut self, cx: &mut Cx, by: isize) {
        let Some(next) = self.picked.checked_add_signed(by).filter(|next| *next < self.entries.len()) else { return };
        self.choose(cx, next);
    }

    fn leave(&mut self, cx: &mut Cx, request: Request) {
        if self.leaving {
            return;
        }
        self.pending = Some(request);
        self.leaving = true;
        self.strip.leave();
        let leave_message =
            cx.rt.package(cx.package).is_some_and(|package| package.responses_to(ids::LEAVE_SCREEN).is_some());
        if leave_message {
            cx.rt.post_to_package(cx.package, ids::LEAVE_SCREEN);
            return;
        }
        // Garage packages expose the authored exit script instead of MainMenu's LEAVE_SCREEN response.
        cx.rt.post_to_package(cx.package, 0x97bd_b465);
        let script = cx
            .object(ids::EVENT_HANDLER)
            .and_then(|object| {
                cx.rt
                    .package(cx.package)?
                    .objects
                    .get(object.index)?
                    .scripts
                    .iter()
                    .find(|script| script.id == ids::SCRIPT_FORWARD)
            })
            .is_some();
        if script {
            cx.script(ids::EVENT_HANDLER, ids::SCRIPT_FORWARD);
            return;
        }
        if let Some(request) = self.pending.take() {
            self.handle.request(request);
        }
    }

    fn accept(&mut self, cx: &mut Cx) {
        if self.leaving {
            return;
        }
        if self.row().is_some() && self.handle.model().selection_locked {
            cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE);
            return;
        }
        let Some(item) = self.entries.get(self.picked).filter(|item| item.enabled) else {
            cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE);
            return;
        };
        if let Some(page) = item.next {
            self.leave(cx, Request::Push(page));
            return;
        }
        if matches!(self.page, Page::VinylGroups { .. }) && item.option == Some(0) {
            self.handle.preview(crate::garage::menu_rows::Row::Part(Part::Vinyl), 0);
            self.handle.add_to_cart();
            self.leave(cx, Request::Pop);
            return;
        }
        if self.row() == Some(crate::garage::menu_rows::Row::Part(Part::Vinyl)) {
            let model = self.handle.model();
            let recolors = model
                .selected()
                .and_then(|car| car.parts.as_ref())
                .and_then(|catalog| catalog.vinyl_remaps.get(&model.draft.parts.parts[Part::Vinyl as usize]?))
                .copied()
                .unwrap_or(0);
            if recolors > 0 {
                self.leave(cx, Request::Push(Page::Paint { color: 2, backroom: self.page.backroom() }));
                return;
            }
        }
        if self.row() == Some(crate::garage::menu_rows::Row::Gauge) {
            self.leave(cx, Request::Push(Page::GaugeColors { backroom: self.page.backroom() }));
            return;
        }
        self.handle.add_to_cart();
        self.leave(cx, Request::Pop);
    }

    fn back(&mut self, cx: &mut Cx) {
        if self.row().is_some() {
            let cart = self.handle.0.lock().unwrap().cart.clone();
            self.handle.restore(cart);
        }
        if matches!(self.page, Page::Categories { .. }) {
            let changed = {
                let state = self.handle.0.lock().unwrap();
                state.cart != state.baseline
            };
            if changed {
                self.handle.show_cart();
                return;
            }
        }
        if self.page == Page::Main {
            let state = self.handle.0.lock().unwrap();
            if state.model.backroom {
                drop(state);
                self.handle.request(Request::Backroom(false));
                self.leave(cx, Request::Replace(Page::Main));
                return;
            }
            let request = match (state.cart != state.baseline, state.model.custom_slots_enabled) {
                (true, _) => Request::Push(Page::Leave),
                (false, true) => Request::Pop,
                (false, false) => Request::event(crate::garage::MenuEvent::Cancel),
            };
            drop(state);
            self.leave(cx, request);
            return;
        }
        self.leave(cx, Request::Pop);
    }
}

impl ScreenLogic for Carousel {
    fn covers_screens(&self) -> bool {
        true
    }
    fn start(&mut self, cx: &mut Cx) {
        if self.row() == Some(crate::garage::menu_rows::Row::Gauge) {
            super::gauge_preview::show(cx);
        }
        if self.page == Page::Main {
            let message = match self.handle.model().backroom {
                true => 0xa1ca_ff8d,
                false => 0x005c_01c5,
            };
            cx.rt.post_to_package(cx.package, message);
        }
        self.strip.build(cx);
        self.picked = cx.memory.get(&format!("garage:{:?}", self.page)).copied().unwrap_or(0);
        if let Some(row) = self.row() {
            let model = self.handle.model();
            let current = row.options(&model).1;
            self.picked = self.entries.iter().position(|item| item.option == Some(current)).unwrap_or(0);
            if let crate::garage::menu_rows::Row::Performance(category) = row
                && model.draft.performance.has_junkman(category)
                && let Some(at) = self
                    .entries
                    .iter()
                    .position(|item| item.row == Some(crate::garage::menu_rows::Row::Junkman(category)))
            {
                self.picked = at;
            }
        }
        if matches!(self.page, Page::RimBrands { .. } | Page::VinylGroups { .. }) {
            let model = self.handle.model();
            let catalog = model.selected().and_then(|car| car.parts.as_ref());
            let wheel = model.draft.parts.parts[Part::Wheels as usize]
                .and_then(|hash| catalog?.rim_metadata.get(&hash))
                .map(|(brand, _)| *brand);
            let vinyl =
                model.draft.parts.parts[Part::Vinyl as usize].and_then(|hash| catalog?.part_groups.get(&hash)).copied();
            self.picked = self
                .entries
                .iter()
                .position(|item| match item.next {
                    Some(Page::Rims { brand, .. }) => wheel == Some(brand),
                    Some(Page::Vinyls { group, .. }) => vinyl == Some(group),
                    _ => false,
                })
                .unwrap_or(0);
        }
        self.picked = self.picked.min(self.entries.len().saturating_sub(1));
        self.strip.open(cx, self.strip.first() + self.picked);
        cx.rt.set_focus(cx.package, self.strip.focus_guid());
        if let Some(row) = self.row() {
            let option = self.entries.get(self.picked).and_then(|item| item.option).unwrap_or(0);
            self.handle.preview(row, option);
        }
        self.refresh(cx);
        self.handle.enter(self.page);
    }

    fn resume(&mut self, cx: &mut Cx) {
        if self.row() == Some(crate::garage::menu_rows::Row::Gauge) {
            super::gauge_preview::show(cx);
        }
        self.leaving = false;
        self.pending = None;
        self.strip.restore();
        self.restore_filter(cx);
        cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE);
        if let Some(row) = self.row() {
            let option = self.entries.get(self.picked).and_then(|item| item.option).unwrap_or(0);
            self.handle.preview(row, option);
        }
        self.refresh(cx);
        self.handle.enter(self.page);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if message == ids::EXIT_COMPLETE {
            if let Some(request) = self.pending.take() {
                self.handle.request(request);
            }
            return;
        }
        if self.leaving {
            return;
        }
        if message == blackbox_feng::ids::PAD_BUTTON0 + 5
            && self.page == Page::Main
            && choices::can_enter_backroom(&self.handle.model())
        {
            self.handle.request(Request::Backroom(true));
            self.leave(cx, Request::Replace(Page::Main));
            return;
        }
        match message {
            ids::INIT_COMPLETE => cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE),
            PAD_LEFT => self.shift(cx, -1),
            PAD_RIGHT => self.shift(cx, 1),
            PAD_ACCEPT | BUTTON_PRESSED => self.accept(cx),
            PAD_BACK => self.back(cx),
            PAD_START => {
                self.handle.show_cart();
            }
            blackbox_feng::ids::PAD_LTRIGGER => self.step_filter(cx, -1),
            blackbox_feng::ids::PAD_RTRIGGER => self.step_filter(cx, 1),
            _ => {}
        }
    }

    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        use blackbox_feng::runtime::pad;
        // Only supplemental actions live here; the package posts the four primary navigation messages.
        if bits & (pad::BUTTON0 << 4) != 0
            && self.page == Page::Main
            && choices::can_enter_backroom(&self.handle.model())
        {
            self.handle.request(Request::Backroom(true));
            self.leave(cx, Request::Replace(Page::Main));
        }
        if bits & (pad::BUTTON0 << 4) != 0
            && matches!(self.page, Page::Categories { section: super::Section::Performance, .. })
            && !self.handle.model().shop
        {
            self.handle.request(Request::Push(Page::MaxPerformance));
        }
        if bits & pad::BUTTON0 != 0 && self.page != Page::Showcase {
            self.leave(cx, Request::Push(Page::Showcase));
        }
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        if self.handle.recover(cx, &mut self.failure) {
            self.pending = None;
            self.leaving = false;
            self.strip.restore();
        }
        self.strip.tick(dt);
        self.strip.place(cx);
        self.refresh(cx);
        // Some specialized packages run their leave script without posting EXIT_COMPLETE.
        if self.leaving
            && self.strip.is_empty()
            && let Some(request) = self.pending.take()
        {
            self.handle.request(request);
        }
    }

    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if self.filter_click(cx, point) {
            return true;
        }
        let Some(hit) = self.strip.hit(cx, point) else { return false };
        if let Hit::Icon(index) = hit {
            if index == self.picked {
                self.accept(cx);
                return true;
            }
            self.choose(cx, index);
        }
        true
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        for _ in 0..steps.unsigned_abs() {
            self.shift(
                cx,
                match steps > 0 {
                    true => -1,
                    false => 1,
                },
            );
        }
    }

    fn footer(&self) -> Footer {
        match self.page {
            Page::Showcase => Footer::Orbit,
            Page::Main => Footer::GarageMain { shop: choices::can_enter_backroom(&self.handle.model()) },
            Page::Categories { section, .. } => Footer::GarageCategories {
                max_performance: section == super::Section::Performance && !self.handle.model().shop,
            },
            _ if self.row().is_some() => Footer::GarageParts,
            _ => Footer::GarageSelect,
        }
    }
}

impl Carousel {
    fn row(&self) -> Option<crate::garage::menu_rows::Row> {
        if let Some(row) = self.entries.get(self.picked).and_then(|item| item.row) {
            return Some(row);
        }
        match self.page {
            Page::Choices { row, .. } => Some(row),
            Page::Rims { .. } => Some(crate::garage::menu_rows::Row::Part(Part::Wheels)),
            Page::Spoilers { .. } => Some(crate::garage::menu_rows::Row::Part(Part::Spoiler)),
            Page::Vinyls { .. } => Some(crate::garage::menu_rows::Row::Part(Part::Vinyl)),
            _ => None,
        }
    }
}

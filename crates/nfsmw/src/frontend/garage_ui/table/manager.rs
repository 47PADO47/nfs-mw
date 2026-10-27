//! MyCarsManager's five-by-two array and the original add-car selector.

use super::super::inspection::{manufacturer, pictures};
use super::super::{GarageUi, Page, Request, chrome};
use crate::frontend::logic::{Cx, Footer, ScreenLogic};
use crate::garage::{Category, MenuEvent, Selection, menu_rows::Row};
use blackbox_feng::{ObjectRef, fe_hash_upper, ids::*};
use glam::Vec2;

pub(super) struct Manager {
    handle: GarageUi,
    page: Page,
    picked: usize,
    first_row: usize,
    objects: Vec<ObjectRef>,
    done: bool,
    failure: u64,
    mode: Option<bool>,
    entry: Selection,
    entry_slot: Option<String>,
}

impl Manager {
    pub fn new(handle: GarageUi, page: Page) -> Self {
        let failure = handle.failure();
        let model = handle.model();
        Self {
            handle,
            page,
            picked: 0,
            first_row: 0,
            objects: Vec::new(),
            done: false,
            failure,
            mode: None,
            entry: model.draft,
            entry_slot: model.custom_slot,
        }
    }

    fn len(&self) -> usize {
        let model = self.handle.model();
        match self.page {
            Page::Manager => model.custom_slots.len() + usize::from(model.custom_slots.len() < model.custom_slot_limit),
            _ => model.cars.iter().filter(|car| car.category == Category::Racing).count(),
        }
    }

    fn add_tile(&self) -> bool {
        let model = self.handle.model();
        self.page == Page::Manager && model.custom_slots.len() < model.custom_slot_limit
    }

    fn slot(&self, index: usize) -> Option<(String, String)> {
        self.handle.model().custom_slots.get(index).cloned()
    }

    fn choose(&mut self, cx: &mut Cx, index: usize) {
        if index >= self.len() {
            return;
        }
        self.picked = index;
        if self.page == Page::Manager
            && let Some((id, _)) = self.slot(index)
        {
            self.handle.request(Request::event(MenuEvent::SelectSlot(id)));
        }
        if self.page == Page::AddCar {
            let model = self.handle.model();
            if let Some(car) = model.cars.iter().filter(|car| car.category == Category::Racing).nth(index) {
                let at = model.cars.iter().position(|entry| entry.folder == car.folder).unwrap_or(0);
                self.handle.preview(Row::Car, at);
            }
        }
        self.refresh(cx);
    }

    fn shift(&mut self, cx: &mut Cx, by: isize) {
        if self.page != Page::Manager {
            let len = self.len();
            if len == 0 {
                return;
            }
            let index = (self.picked as isize + by).rem_euclid(len as isize) as usize;
            self.choose(cx, index);
            return;
        }
        let len = self.len();
        if len == 0 {
            return;
        }
        let start = self.picked / 5 * 5;
        let column = self.picked % 5;
        let index = match by {
            -1 | 1 => start + (column as isize + by).rem_euclid((len - start).min(5) as isize) as usize,
            -5 | 5 => {
                let Some(row) = (self.picked / 5).checked_add_signed(by / 5) else { return };
                if row * 5 >= len {
                    return;
                }
                (row * 5 + column).min(len - 1)
            }
            _ => return,
        };
        self.choose(cx, index);
    }

    fn align_window(&mut self) {
        if self.page != Page::Manager {
            return;
        }
        let len = self.len();
        self.picked = self.picked.min(len.saturating_sub(1));
        let row = self.picked / 5;
        self.first_row = self.first_row.clamp(row.saturating_sub(1), row).min(len.div_ceil(5).saturating_sub(2));
    }

    fn refresh_scrollbar(&self, cx: &mut Cx) {
        let rows = self.len().div_ceil(5);
        let shown = rows > 2;
        for name in ["scrollbar_Backing", "scrollbar_Handle", "scrollbar_Arrow_1", "scrollbar_Arrow_2"] {
            if let Some(object) = cx.named(name) {
                cx.rt.set_hidden(object, !shown);
            }
        }
        if !shown {
            return;
        }
        let (Some(backing), Some(handle)) = (cx.named("scrollbar_Backing"), cx.named("scrollbar_Handle")) else {
            return;
        };
        let (Some(at), Some(size), Some(width)) = (cx.rt.position(backing), cx.rt.size(backing), cx.rt.size(handle))
        else {
            return;
        };
        let height = size.y * 2.0 / rows as f32;
        let top = at.y - size.y * 0.5;
        let travel = (size.y - height) / (rows - 2) as f32;
        cx.rt.set_size_xy(handle, width.x, height);
        cx.rt.set_position_xy(handle, at.x, top + self.first_row as f32 * travel + height * 0.5);
        for (name, dim) in [("scrollbar_Arrow_1", self.picked < 5), ("scrollbar_Arrow_2", self.picked / 5 == rows - 1)]
        {
            if let Some(object) = cx.named(name) {
                let script = fe_hash_upper(match dim {
                    true => "DIM",
                    false => "NORMAL",
                });
                if cx.rt.script_of(object) != Some(script) {
                    cx.rt.run_script(object, script);
                }
            }
        }
    }

    fn click_scrollbar(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if self.page != Page::Manager || self.done {
            return false;
        }
        if self.click_scrollbar_track(cx, point) {
            return true;
        }
        let mut controls = Vec::new();
        for (name, by) in [("scrollbar_Arrow_1", -5), ("scrollbar_Arrow_2", 5)] {
            let Some(group) = cx.named(name) else { continue };
            let Some(package) = cx.rt.package(cx.package) else { continue };
            let guid = package.objects[group.index].guid;
            controls.push((group, by));
            controls.extend(
                package
                    .objects
                    .iter()
                    .enumerate()
                    .filter(|(_, object)| object.parent == Some(guid))
                    .map(|(index, _)| (ObjectRef { package: cx.package, index }, by)),
            );
        }
        let objects: Vec<_> = controls.iter().map(|(object, _)| *object).collect();
        let Some(hit) = cx.rt.object_at(&objects, point) else { return false };
        let Some((_, by)) = controls.iter().find(|(object, _)| *object == hit) else { return false };
        self.shift(cx, *by);
        true
    }

    fn click_scrollbar_track(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        let (Some(backing), Some(handle)) = (cx.named("scrollbar_Backing"), cx.named("scrollbar_Handle")) else {
            return false;
        };
        let Some(hit) = cx.rt.object_at(&[backing, handle], point) else { return false };
        if hit == handle {
            self.accept();
            return true;
        }
        let Some(quad) = cx.rt.object_quad(handle) else { return false };
        let edges = [(-0.5, -0.5), (0.5, -0.5), (-0.5, 0.5), (0.5, 0.5)]
            .map(|(x, y)| quad.transform_point3(glam::Vec3::new(x, y, 0.0)).y);
        let top = edges.into_iter().fold(f32::INFINITY, f32::min);
        let bottom = edges.into_iter().fold(f32::NEG_INFINITY, f32::max);
        let by = match (point.y < top, point.y > bottom) {
            (true, _) => -5,
            (_, true) => 5,
            _ => return false,
        };
        self.shift(cx, by);
        true
    }

    fn refresh(&mut self, cx: &mut Cx) {
        self.align_window();
        let model = self.handle.model();
        self.handle.header(cx, self.page);
        super::super::details::bars(cx, &model);
        if let Some(car) = model.selected() {
            let (maker, name) = pictures(cx, car);
            for (hash, texture) in [(0x3e01_ad1d, maker), (0xb05d_d708, name)] {
                if let Some(object) = cx.object(hash) {
                    cx.rt.set_texture(object, texture);
                }
            }
        }
        if self.page == Page::Manager {
            hide_sample_names(cx);
            let base = self.first_row * 5;
            self.refresh_scrollbar(cx);
            for (index, object) in self.objects.iter().copied().enumerate() {
                let at = base + index;
                cx.rt.set_hidden(object, at >= self.len());
                let slot = self.slot(at);
                let texture = match slot {
                    Some((id, _)) => model
                        .custom_slot_cars
                        .get(&id)
                        .and_then(|folder| model.cars.iter().find(|car| &car.folder == folder))
                        .map(|car| manufacturer(cx, &car.manufacturer))
                        .unwrap_or(0),
                    None => 0x91d3_a6ba,
                };
                cx.rt.set_texture(object, texture);
                chrome::focus(cx, object, at == self.picked);
                if at == self.picked {
                    cx.rt.focus_button(object);
                }
            }
            let label = self.slot(self.picked).map(|(_, label)| label).unwrap_or_else(|| "Add New Car".into());
            chrome::selection(cx, &label, self.picked + 1, self.len());
            let add = self.add_tile() && self.picked == model.custom_slots.len();
            cx.hide(0x4ed1_e692, !add);
            cx.hide(0x73dc_b662, !add);
            cx.hide(0x5131_9794, add);
            if self.mode != Some(add) {
                let message = match add {
                    true => 0x42ea_22dd,
                    false => 0x06d4_1ccc,
                };
                cx.rt.post_to_package(cx.package, message);
                self.mode = Some(add);
            }
            return;
        }
        let Some(car) = model.cars.iter().filter(|car| car.category == Category::Racing).nth(self.picked) else {
            return;
        };
        chrome::selection(cx, &car.name, self.picked + 1, self.len());
        let (maker, texture) = pictures(cx, car);
        for (name, texture) in [(0x3e01_ad1d, maker), (0xb05d_d708, texture)] {
            if let Some(object) = cx.object(name) {
                cx.rt.set_hidden(object, false);
                cx.rt.set_texture(object, texture);
            }
        }
        hide_sample_names(cx);
        cx.hide(0xd0f7_c7cc, car.unavailable.is_none());
        cx.text_object(0x2d25_b2c4, car.unavailable.as_deref().unwrap_or_default());
    }

    fn accept(&mut self) {
        if self.done {
            return;
        }
        if self.page == Page::Manager {
            self.done = true;
            if self.add_tile() && self.picked == self.handle.model().custom_slots.len() {
                self.handle.request(Request::Push(Page::AddCar));
                return;
            }
            if let Some((id, _)) = self.slot(self.picked) {
                self.handle.request(Request::event(MenuEvent::SelectSlot(id)));
            }
            self.handle.request(Request::Push(Page::Main));
            return;
        }
        let model = self.handle.model();
        if model.selected().is_none_or(|car| car.unavailable.is_some()) {
            return;
        }
        self.done = true;
        self.handle.request(Request::event(MenuEvent::AddSlot));
        self.handle.request(Request::Replace(Page::Main));
    }
}

impl ScreenLogic for Manager {
    fn covers_screens(&self) -> bool {
        true
    }
    fn start(&mut self, cx: &mut Cx) {
        if self.page == Page::Manager {
            self.objects = (1..=10).filter_map(|index| cx.named(&format!("CAR_ICON_{index}"))).collect();
            let model = self.handle.model();
            self.picked = model
                .custom_slot
                .as_ref()
                .and_then(|id| model.custom_slots.iter().position(|(slot, _)| slot == id))
                .unwrap_or(0);
        }
        if self.page == Page::AddCar {
            let model = self.handle.model();
            self.picked = model
                .cars
                .iter()
                .filter(|car| car.category == Category::Racing)
                .position(|car| car.folder == model.draft.car)
                .unwrap_or(0);
            self.choose(cx, self.picked);
        }
        self.refresh(cx);
        self.handle.enter(self.page);
    }
    fn resume(&mut self, cx: &mut Cx) {
        self.done = false;
        self.mode = None;
        // Showcase's global leave fades this cached manager's scrollbar parent.
        if self.page == Page::Manager
            && let Some(group) = cx.object(0x8b58_33c3)
        {
            cx.rt.run_script(group, 0x5079_c8f8);
        }
        let model = self.handle.model();
        if self.page == Page::Manager
            && let Some(at) =
                model.custom_slot.as_ref().and_then(|id| model.custom_slots.iter().position(|(slot, _)| slot == id))
        {
            self.picked = at;
        }
        self.picked = self.picked.min(self.len().saturating_sub(1));
        self.refresh(cx);
        self.handle.enter(self.page);
    }
    fn message(&mut self, cx: &mut Cx, message: u32) {
        if self.done {
            return;
        }
        match message {
            PAD_LEFT => self.shift(cx, -1),
            PAD_RIGHT => self.shift(cx, 1),
            PAD_UP if self.page == Page::Manager => self.shift(cx, -5),
            PAD_DOWN if self.page == Page::Manager => self.shift(cx, 5),
            PAD_ACCEPT | BUTTON_PRESSED => self.accept(),
            PAD_BACK => {
                self.done = true;
                if self.page == Page::AddCar {
                    if let Some(id) = self.entry_slot.clone() {
                        self.handle.request(Request::event(MenuEvent::SelectSlot(id)));
                    }
                    self.handle.restore(self.entry.clone());
                }
                let request = match self.page {
                    Page::Manager => Request::event(MenuEvent::Cancel),
                    _ => Request::Pop,
                };
                self.handle.request(request);
            }
            _ => {}
        }
    }
    fn pad_pressed(&mut self, _cx: &mut Cx, bits: u32) {
        use blackbox_feng::runtime::pad;
        if self.done {
            return;
        }
        if bits & (pad::BUTTON0 << 5) != 0 && self.slot(self.picked).is_some() {
            self.handle.request(Request::Push(Page::Delete));
        }
        if bits & pad::BUTTON0 != 0 {
            self.handle.request(Request::Push(Page::Showcase));
        }
    }
    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if self.click_scrollbar(cx, point) {
            return true;
        }
        let Some(hit) = cx.rt.object_at(&self.objects, point) else { return false };
        let Some(index) = self.objects.iter().position(|object| *object == hit) else { return false };
        let index = self.first_row * 5 + index;
        if index == self.picked {
            self.accept();
            return true;
        }
        self.choose(cx, index);
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
    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        if self.handle.recover(cx, &mut self.failure) {
            self.done = false;
        }
        self.refresh(cx);
    }
    fn footer(&self) -> Footer {
        match self.page {
            Page::Manager => Footer::GarageManager { add: self.slot(self.picked).is_none() },
            _ => Footer::GarageManager { add: true },
        }
    }
}

/// Hide the two name strings of the `Car_Select` package: they hold the sample text `temp123` and `tempXXX`, and the screens
/// name a car with the two pictures instead (`inspection::pictures`).
fn hide_sample_names(cx: &mut Cx) {
    let names: Vec<_> = cx
        .rt
        .package(cx.package)
        .into_iter()
        .flat_map(|package| package.objects.iter().enumerate())
        .filter(|(_, object)| matches!(object.name_hash, 0xd6d3_2016 | 0x79d6_e45c))
        .map(|(index, _)| ObjectRef { package: cx.package, index })
        .collect();
    for object in names {
        cx.rt.set_hidden(object, true);
        cx.rt.set_text(object, String::new());
    }
}

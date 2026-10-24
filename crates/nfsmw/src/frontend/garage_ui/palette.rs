//! The original Paint.fng eighty-swatch grid, populated with installed colours.

use super::{GarageUi, Page, Request, chrome};
use crate::frontend::icon_strip::{Face, Hit, IconStrip, WHITE};
use crate::frontend::logic::{Cx, Footer, ScreenLogic};
use crate::garage::{Selection, menu_rows::Row};
use blackbox_feng::{ObjectRef, ids::*};
use glam::Vec2;
use nfsmw_data::car::customization::Category as Part;

struct Swatch {
    hash: Option<u32>,
    rgb: [u8; 3],
    option: usize,
}

pub(super) struct Palette {
    handle: GarageUi,
    page: Page,
    entry: Selection,
    families: Vec<String>,
    family: usize,
    family_cursors: Vec<usize>,
    colors: Vec<Swatch>,
    picked: usize,
    objects: Vec<ObjectRef>,
    done: bool,
    channel: usize,
    channels: usize,
    cursors: [usize; 3],
    layers: Option<IconStrip>,
}

impl Palette {
    pub fn new(handle: GarageUi, page: Page) -> Self {
        let model = handle.model();
        let mut families: Vec<String> = Vec::new();
        if let Some(catalog) = model.selected().and_then(|car| car.parts.as_ref()) {
            for (_, hash) in &catalog.finishes {
                if let Some(family) = catalog.paint_families.get(hash).filter(|family| !families.contains(*family)) {
                    families.push(family.clone());
                }
            }
            for choice in &catalog.choices[Part::Paint as usize] {
                if let Some(family) =
                    catalog.paint_families.get(&choice.hash).filter(|family| !families.contains(*family))
                {
                    families.push(family.clone());
                }
            }
        }
        if families.is_empty() {
            families.push(String::new());
        }
        let family = model.draft.parts.parts[Part::Paint as usize]
            .and_then(|hash| {
                let family = model.selected()?.parts.as_ref()?.paint_families.get(&hash)?;
                families.iter().position(|candidate| candidate == family)
            })
            .unwrap_or(0);
        let mut channels = model
            .selected()
            .and_then(|car| car.parts.as_ref())
            .and_then(|catalog| catalog.vinyl_remaps.get(&model.draft.parts.parts[Part::Vinyl as usize]?))
            .copied()
            .unwrap_or(1)
            .clamp(1, 3);
        if matches!(page, Page::GaugeColors { .. }) {
            channels = 3;
        }
        let family_cursors = vec![0; families.len()];
        Self {
            entry: model.draft,
            handle,
            page,
            families,
            family,
            family_cursors,
            colors: Vec::new(),
            picked: 0,
            objects: Vec::new(),
            done: false,
            channel: 0,
            channels,
            cursors: [0; 3],
            layers: None,
        }
    }

    fn color(&self) -> usize {
        match self.page {
            Page::GaugeColors { .. } => [6, 7, 5][self.channel],
            Page::Paint { color: 2..=4, .. } => 2 + self.channel,
            Page::Paint { color, .. } => color,
            _ => 0,
        }
    }

    fn rebuild(&mut self, cx: &mut Cx) {
        let model = self.handle.model();
        self.colors.clear();
        let Some(car) = model.selected() else { return };
        let Some(catalog) = &car.parts else { return };
        if self.color() >= 2 {
            self.colors.extend(catalog.color_palettes[self.color() - 2].iter().map(|color| Swatch {
                hash: Some(color.hash),
                rgb: color.rgb,
                option: 0,
            }));
            let rgb = Row::Color(self.color()).color(&model);
            let selected = match self.color() {
                color @ 2..=4 => model.draft.parts.vinyl_color_parts[color - 2],
                color @ 5..=7 => model.draft.gauges.color_parts[color - 5],
                _ => None,
            };
            self.picked = self
                .colors
                .iter()
                .position(|swatch| selected.map_or(rgb == Some(swatch.rgb), |hash| swatch.hash == Some(hash)))
                .unwrap_or(0);
            self.refresh(cx);
            return;
        }
        let category = match self.color() {
            1 => Part::RimPaint,
            _ => Part::Paint,
        };
        let family = &self.families[self.family];
        if let Some(hash) = catalog.stock[category as usize]
            && (self.color() == 1 || catalog.paint_families.get(&hash) == Some(family))
            && let Some(rgb) = catalog.colors.get(&hash).copied()
        {
            self.colors.push(Swatch { hash: None, rgb, option: 0 });
        }
        for (index, choice) in catalog.choices[category as usize].iter().enumerate() {
            if self.color() == 0 && catalog.paint_families.get(&choice.hash) != Some(family) {
                continue;
            }
            let Some(rgb) = catalog.colors.get(&choice.hash).copied() else { continue };
            self.colors.push(Swatch { hash: Some(choice.hash), rgb, option: index + 1 });
        }
        let selected = match self.color() {
            0 => model.draft.parts.parts[Part::Paint as usize],
            1 => model.draft.parts.parts[Part::RimPaint as usize],
            _ => None,
        };
        let rgb = Row::Color(self.color()).color(&model);
        self.picked = self
            .colors
            .iter()
            .position(|swatch| selected.is_some_and(|hash| swatch.hash == Some(hash)) || rgb == Some(swatch.rgb))
            .unwrap_or(0);
        self.refresh(cx);
    }

    fn refresh(&self, cx: &mut Cx) {
        self.handle.header(cx, self.page);
        if self.color() == 1 || (matches!(self.page, Page::Paint { color: 2..=4, .. }) && self.channels == 1) {
            for hash in [0x2c52_6172, 0x2c3c_c2d3, 0x5363_9a10, 0x7800_8599] {
                cx.hide(hash, true);
            }
        }
        let count = self.objects.len().max(1);
        let base = self.picked / count * count;
        for (index, object) in self.objects.iter().copied().enumerate() {
            let swatch = self.colors.get(base + index);
            cx.rt.set_hidden(object, swatch.is_none());
            if let Some(swatch) = swatch {
                cx.rt.set_colour_rgba(object, [swatch.rgb[0], swatch.rgb[1], swatch.rgb[2], 255]);
            }
        }
        let family = self.family_label();
        chrome::text(cx, "FINISH_NAME", &family);
        cx.text_object(0x7800_8599, &family);
        if matches!(self.page, Page::Paint { color: 2..=4, .. }) {
            let key = match (self.channels, self.channel) {
                (1, _) => 0xd8ee_1a80,
                (2, 0) => 0x5198_ba16,
                (2, _) => 0x5198_be57,
                (3, 0) => 0x5198_ba17,
                (3, 1) => 0x5198_be58,
                _ => 0x5198_c299,
            };
            if let Some(object) = cx.object(0x7800_8599) {
                cx.rt.set_label(object, key);
            }
        }
        chrome::selection(cx, &family, self.picked + 1, self.colors.len());
        if let Some(layers) = &self.layers {
            layers.place(cx);
            chrome::selection(
                cx,
                ["Needle", "Characters", "Backing"][self.channel],
                self.picked + 1,
                self.colors.len(),
            );
            super::gauge_preview::refresh(cx, &self.handle.model());
        }
        if let Some(swatch) = self.colors.get(self.picked) {
            let state = self.handle.0.lock().unwrap();
            let row = match self.color() {
                0 => Row::Part(Part::Paint),
                1 => Row::Part(Part::RimPaint),
                color => Row::Color(color),
            };
            match self.color() {
                0 | 1 => super::details::refresh(cx, &state.model, row, swatch.option, &state.baseline, &state.cart),
                _ => super::details::color_status(
                    cx,
                    row,
                    swatch.rgb,
                    swatch.hash,
                    &state.model,
                    &state.baseline,
                    &state.cart,
                ),
            }
        }
        if let Some(object) = self.objects.get(self.picked % count).copied() {
            cx.rt.focus_button(object);
        }
    }

    fn family_label(&self) -> String {
        let family = &self.families[self.family];
        let label = (self.color() == 0 && matches!(family.as_str(), "GLOSS" | "METAL" | "PEARL"))
            .then(|| {
                let model = self.handle.model();
                let catalog = model.selected()?.parts.as_ref()?;
                let choice = catalog.choices[Part::Paint as usize]
                    .iter()
                    .find(|choice| catalog.paint_families.get(&choice.hash) == Some(family))?;
                let (label, _) = catalog.paint_names.get(&choice.hash)?;
                crate::ui::Text::Game(*label).resolve()
            })
            .flatten();
        label.unwrap_or_else(|| family.clone())
    }

    fn choose(&mut self, cx: &mut Cx, at: usize) {
        let Some(swatch) = self.colors.get(at) else { return };
        self.picked = at;
        let color = self.color();
        let mut state = self.handle.0.lock().unwrap();
        if color <= 1 {
            let part = match color {
                0 => Part::Paint,
                _ => Part::RimPaint,
            };
            Row::Part(part).set(&mut state.model, swatch.option);
            Row::Color(color).set_color(&mut state.model, None);
        }
        if color > 1 {
            Row::Color(color).set_color(&mut state.model, Some(swatch.rgb));
            match color {
                2..=4 => state.model.draft.parts.vinyl_color_parts[color - 2] = swatch.hash,
                5..=7 => state.model.draft.gauges.color_parts[color - 5] = swatch.hash,
                _ => {}
            }
        }
        let selection = state.model.draft.clone();
        state.requests.push(Request::event(crate::garage::MenuEvent::Preview(selection)));
        drop(state);
        self.refresh(cx);
    }

    fn shift(&mut self, cx: &mut Cx, by: isize) {
        if self.colors.is_empty() {
            return;
        }
        if matches!(self.page, Page::GaugeColors { .. }) {
            self.choose(cx, (self.picked as isize + by).rem_euclid(self.colors.len() as isize) as usize);
            return;
        }
        if by.unsigned_abs() == 1 {
            let first = self.picked / 20 * 20;
            let count = (self.colors.len() - first).min(20);
            self.choose(cx, first + (self.picked as isize - first as isize + by).rem_euclid(count as isize) as usize);
            return;
        }
        if by == 20 && self.picked / 20 < (self.colors.len() - 1) / 20 {
            self.choose(cx, (self.picked + 20).min(self.colors.len() - 1));
            return;
        }
        let Some(next) = self.picked.checked_add_signed(by).filter(|next| *next < self.colors.len()) else { return };
        self.choose(cx, next);
    }

    fn change_finish(&mut self, cx: &mut Cx, by: isize) {
        if self.color() == 1 {
            return;
        }
        if matches!(self.page, Page::Paint { color: 2..=4, .. } | Page::GaugeColors { .. }) {
            self.cursors[self.channel] = self.picked;
            self.channel = (self.channel as isize + by).rem_euclid(self.channels as isize) as usize;
            self.rebuild(cx);
            self.picked = self.cursors[self.channel].min(self.colors.len().saturating_sub(1));
            self.choose(cx, self.picked);
            if let Some(layers) = &mut self.layers {
                layers.select(cx, layers.first() + self.channel);
            }
            return;
        }
        let next = (self.family as isize + by).rem_euclid(self.families.len() as isize) as usize;
        self.family_cursors[self.family] = self.picked;
        self.family = next;
        self.rebuild(cx);
        self.picked = self.family_cursors[next].min(self.colors.len().saturating_sub(1));
        self.choose(cx, self.picked);
    }

    fn close(&mut self, save: bool) {
        if self.done {
            return;
        }
        if save && self.handle.model().selection_locked {
            return;
        }
        self.done = true;
        if save {
            self.handle.add_to_cart();
        }
        if !save {
            self.handle.restore(self.entry.clone());
        }
        self.handle.request(Request::Pop);
    }
}

impl ScreenLogic for Palette {
    fn covers_screens(&self) -> bool {
        true
    }
    fn start(&mut self, cx: &mut Cx) {
        if matches!(self.page, Page::GaugeColors { .. }) {
            super::gauge_preview::show(cx);
        }
        self.objects = (1..=80).filter_map(|index| cx.named(&format!("Color_{index}"))).collect();
        if matches!(self.page, Page::GaugeColors { .. }) {
            let mut layers = IconStrip::new(
                [0x70f5_6628, 0xbf66_82c9, 0xcc9e_1ce4].into_iter().map(|texture| Face::new(texture, true)).collect(),
                WHITE,
            );
            layers.build(cx);
            layers.open(cx, layers.first());
            self.layers = Some(layers);
        }
        self.rebuild(cx);
        if matches!(self.page, Page::Paint { color: 2..=4, .. } | Page::GaugeColors { .. }) {
            for channel in 0..self.channels {
                self.channel = channel;
                self.rebuild(cx);
                self.cursors[channel] = self.picked;
                self.choose(cx, self.picked);
            }
            self.channel = 0;
            self.rebuild(cx);
        }
        if matches!(self.page, Page::Paint { color: 2..=4, .. }) {
            for hash in [0x2c3c_c2d3, 0x5363_9a10] {
                cx.hide(hash, self.channels < 2);
            }
            if self.channels > 1 {
                cx.rt.post_to_package(cx.package, 0x1a72_40f3);
            }
        }
        self.handle.enter(self.page);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if self.done {
            return;
        }
        if matches!(self.page, Page::GaugeColors { .. }) {
            match message {
                PAD_LEFT => {
                    self.change_finish(cx, -1);
                    return;
                }
                PAD_RIGHT => {
                    self.change_finish(cx, 1);
                    return;
                }
                PAD_UP => {
                    self.shift(cx, -1);
                    return;
                }
                PAD_DOWN => {
                    self.shift(cx, 1);
                    return;
                }
                _ => {}
            }
        }
        match message {
            crate::frontend::ids::INIT_COMPLETE => {
                cx.rt.post_to_package(cx.package, crate::frontend::ids::INPUT_ENABLE)
            }
            PAD_LEFT => self.shift(cx, -1),
            PAD_RIGHT => self.shift(cx, 1),
            PAD_UP => self.shift(cx, -20),
            PAD_DOWN => self.shift(cx, 20),
            PAD_LTRIGGER => self.change_finish(cx, -1),
            PAD_RTRIGGER => self.change_finish(cx, 1),
            PAD_ACCEPT | BUTTON_PRESSED => {
                self.choose(cx, self.picked);
                self.close(true);
            }
            PAD_BACK => self.close(false),
            PAD_START => {
                self.handle.show_cart();
            }
            _ => {}
        }
    }

    fn resume(&mut self, cx: &mut Cx) {
        if matches!(self.page, Page::GaugeColors { .. }) {
            super::gauge_preview::show(cx);
        }
        if matches!(self.page, Page::Paint { color: 0 | 2..=4, .. })
            && let Some(group) = cx.object(0x2c52_6172)
        {
            cx.rt.run_script(group, 0x5079_c8f8);
        }
        self.done = false;
        self.refresh(cx);
        self.handle.enter(self.page);
    }
    fn pad_pressed(&mut self, cx: &mut Cx, bits: u32) {
        // The native HUD swatches have no BUTTON_PRESSED/PAD_ACCEPT response.
        if matches!(self.page, Page::GaugeColors { .. }) && bits & blackbox_feng::runtime::pad::ACCEPT != 0 {
            self.choose(cx, self.picked);
            self.close(true);
        }
        if bits & blackbox_feng::runtime::pad::BUTTON0 != 0 {
            self.handle.request(Request::Push(Page::Showcase));
        }
    }

    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if let Some(layers) = &self.layers
            && let Some(hit) = layers.hit(cx, point)
        {
            if let Hit::Icon(index) = hit {
                let by = index as isize - self.channel as isize;
                self.change_finish(cx, by);
            }
            return true;
        }
        if matches!(self.page, Page::GaugeColors { .. }) {
            for (hashes, by) in [([0x204b_882c, 0x0603_c1d1], -1), ([0x611d_6eae, 0xc6ea_c384], 1)] {
                let arrows: Vec<_> = hashes.into_iter().filter_map(|hash| cx.object(hash)).collect();
                if cx.rt.object_at(&arrows, point).is_some() {
                    self.shift(cx, by);
                    return true;
                }
            }
        }
        for (hashes, by) in [([0x6b36_4f8b, 0xd1cd_a594], -1), ([0x7935_4351, 0x9345_e4a7], 1)] {
            let arrows: Vec<_> = hashes.into_iter().filter_map(|hash| cx.object(hash)).collect();
            if cx.rt.object_at(&arrows, point).is_some() {
                self.change_finish(cx, by);
                return true;
            }
        }
        let Some(hit) = cx.rt.object_at(&self.objects, point) else { return false };
        let Some(index) = self.objects.iter().position(|object| *object == hit) else { return false };
        let count = self.objects.len().max(1);
        self.choose(cx, self.picked / count * count + index);
        true
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        if let Some(layers) = &mut self.layers {
            layers.tick(dt);
        }
        self.refresh(cx);
    }
    fn footer(&self) -> Footer {
        Footer::GarageParts
    }
}

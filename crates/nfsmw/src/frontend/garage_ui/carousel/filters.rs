//! Native decal colors, spoiler families and rim sizes share their existing filter controls.

use super::*;
use crate::garage::menu_rows::Row;

impl Carousel {
    pub(super) fn restore_filter(&self, cx: &mut Cx) {
        let names: &[u32] = match self.page {
            Page::Choices { row: Row::Decal(_, _), .. } | Page::Spoilers { .. } => &[0x5363_9a10, 0x2c3c_c2d3],
            Page::Rims { .. } => &[0x4bf6_87bf],
            _ => return,
        };
        for name in names {
            if let Some(object) = cx.object(*name) {
                cx.rt.run_script(object, 0x5079_c8f8);
            }
        }
    }

    pub(super) fn filter_caption(&self, cx: &mut Cx) {
        if !matches!(self.page, Page::Choices { row: Row::Decal(_, _), .. }) {
            return;
        }
        let captions: Vec<_> = cx
            .rt
            .package(cx.package)
            .into_iter()
            .flat_map(|package| package.objects.iter().enumerate())
            .filter(|(_, object)| {
                object.name_hash == 0x889b_acb6 && object.flags & blackbox_feng::package::flags::IS_BUTTON == 0
            })
            .map(|(index, _)| blackbox_feng::ObjectRef { package: cx.package, index })
            .collect();
        for object in captions {
            cx.rt.set_label(object, self.decal_label());
        }
    }

    fn decal_label(&self) -> u32 {
        match self.decal_black {
            true => 0x41f0_a3a5,
            false => 0x436a_98e9,
        }
    }

    pub(super) fn filter_click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        let decal = matches!(self.page, Page::Choices { row: Row::Decal(_, _), .. });
        if self.leaving || !(decal || matches!(self.page, Page::Rims { .. } | Page::Spoilers { .. })) {
            return false;
        }
        let mut controls: Vec<_> = [(0x6b36_4f8b, -1), (0x7935_4351, 1), (0xd1cd_a594, -1), (0x9345_e4a7, 1)]
            .into_iter()
            .filter_map(|(hash, by)| cx.object(hash).map(|object| (object, by)))
            .collect();
        if decal {
            controls.extend(
                [0xd2c2_1a9c, 0x2c3c_c2d3].into_iter().filter_map(|hash| cx.object(hash).map(|object| (object, 1))),
            );
        }
        let objects: Vec<_> = controls.iter().map(|(object, _)| *object).collect();
        let Some(hit) = cx.rt.object_at(&objects, point) else { return false };
        let Some((_, by)) = controls.iter().find(|(object, _)| *object == hit) else { return false };
        self.step_filter(cx, *by);
        true
    }

    pub(super) fn step_filter(&mut self, cx: &mut Cx, by: isize) {
        if let Page::Choices { row: row @ Row::Decal(_, _), .. } = self.page {
            let model = self.handle.model();
            let option = self.entries.get(self.picked).and_then(|item| item.option).unwrap_or(0);
            let brand = choices::decal::brand(&model, row, option);
            self.decal_black = !self.decal_black;
            self.entries = choices::items(self.page, &model);
            let picked = choices::decal::filter(&model, row, self.decal_black, &mut self.entries, brand);
            self.rebuild_filter(cx, picked);
            return;
        }
        if let Page::Spoilers { group, backroom } = self.page {
            self.filter_cursors[group as usize] = self.picked;
            let next = (isize::from(group) + by).rem_euclid(4) as u8;
            self.page = Page::Spoilers { group: next, backroom };
            self.entries = choices::items(self.page, &self.handle.model());
            self.strip =
                IconStrip::new(self.entries.iter().map(|item| Face::new(item.texture, item.enabled)).collect(), WHITE);
            self.strip.build(cx);
            self.picked = self.filter_cursors[next as usize].min(self.entries.len().saturating_sub(1));
            self.strip.open(cx, self.strip.first() + self.picked);
            self.choose(cx, self.picked);
            return;
        }
        let Page::Rims { brand, radius, backroom } = self.page else { return };
        let radii = choices::radii(&self.handle.model(), brand);
        if radii.is_empty() {
            return;
        }
        let current = radii.iter().position(|candidate| *candidate == radius).unwrap_or(0);
        let at = (current as isize + by).rem_euclid(radii.len() as isize) as usize;
        self.page = Page::Rims { brand, radius: radii[at], backroom };
        self.entries = choices::items(self.page, &self.handle.model());
        self.strip =
            IconStrip::new(self.entries.iter().map(|item| Face::new(item.texture, item.enabled)).collect(), WHITE);
        self.strip.build(cx);
        self.picked = match self.picked < self.entries.len() {
            true => self.picked,
            false => 0,
        };
        self.strip.open(cx, self.strip.first() + self.picked);
        self.choose(cx, self.picked);
    }

    fn rebuild_filter(&mut self, cx: &mut Cx, picked: usize) {
        self.strip =
            IconStrip::new(self.entries.iter().map(|item| Face::new(item.texture, item.enabled)).collect(), WHITE);
        self.strip.build(cx);
        self.picked = picked.min(self.entries.len().saturating_sub(1));
        self.strip.open(cx, self.strip.first() + self.picked);
        self.choose(cx, self.picked);
    }
}

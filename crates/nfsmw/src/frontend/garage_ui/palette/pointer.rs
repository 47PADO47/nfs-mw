//! Pointer interaction for the paint and gauge palettes.

use super::{Cx, Page, Palette, Vec2};
use crate::frontend::icon_strip::Hit;

impl Palette {
    pub(super) fn click_palette(&mut self, cx: &mut Cx, point: Vec2) -> bool {
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
        let Some(hit) = cx.rt.object_at(&self.objects, point) else {
            return false;
        };
        let Some(index) = self.objects.iter().position(|object| *object == hit) else {
            return false;
        };
        let count = self.objects.len().max(1);
        self.choose(cx, self.picked / count * count + index);
        true
    }
}

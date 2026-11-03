//! The existing My Cars chrome stays visible while its catalogue is being read.

use crate::frontend::garage_ui::{GarageUi, Page};
use crate::frontend::logic::{Cx, Footer, ScreenLogic};

pub(super) fn logic() -> Box<dyn ScreenLogic> {
    let ui = GarageUi::new();
    ui.sync(&crate::garage::Model { preview_loading: true, ..Default::default() });
    Box::new(Pending { inner: ui.logic(Page::Manager) })
}

struct Pending {
    inner: Box<dyn ScreenLogic>,
}

impl Pending {
    fn caption(cx: &mut Cx) {
        let text = crate::i18n::text("garage-preview-loading-catalog", &crate::i18n::Args::new());
        for name in [crate::frontend::ids::ICON_TITLE, crate::frontend::ids::ICON_TITLE_SHADOW] {
            cx.text_object(name, &text);
        }
        for name in [0x5b80_5ee5, 0x40d2_3023, 0x2e48_1779, 0x5e7b_2637] {
            cx.hide(name, true);
        }
    }
}

impl ScreenLogic for Pending {
    fn message(&mut self, _cx: &mut Cx, _message: u32) {}
    fn covers_screens(&self) -> bool {
        true
    }
    fn start(&mut self, cx: &mut Cx) {
        self.inner.start(cx);
        Self::caption(cx);
    }
    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        self.inner.tick(cx, dt);
        Self::caption(cx);
    }
    fn footer(&self) -> Footer {
        Footer::GarageLoading
    }
}

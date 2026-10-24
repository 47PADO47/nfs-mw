//! Original garage screens drive the same preview, quote and persistence transactions as the world.

use super::super::garage_ui::{GarageUi, Page, Request};
use super::*;
use crate::garage::{MenuEvent, Overlay};

impl Frontend {
    pub(in crate::frontend) fn open_tuning(&mut self, host: &mut Host, env: &mut Env) {
        if self.garage_ui.is_some() || host.scene.tuning_available() != Some(true) {
            return;
        }
        let Some(renderer) = host.renderer.as_mut() else { return };
        if let Err(error) = host.scene.begin_tuning(renderer.as_mut()) {
            let message = format!("{error:#}");
            self.garage_error(host, error);
            let alert = super::super::confirm::Alert::new(crate::ui::Text::Custom(message));
            let commands = self.screens.open_with(
                super::super::ids::screen::DIALOG,
                Box::new(super::super::confirm::AlertLogic::new(alert)),
                false,
                env,
            );
            self.apply(commands, host, env);
            return;
        }
        let ui = GarageUi::new();
        if let Some(model) = host.scene.garage() {
            ui.sync(model);
        }
        let commands = self.screens.open_with(Page::Tuning.package(), ui.logic(Page::Tuning), false, env);
        self.garage_ui = Some(ui);
        self.garage_tuning = true;
        self.latch = true;
        self.apply(commands, host, env);
    }

    pub(in crate::frontend) fn open_garage(&mut self, host: &mut Host) {
        let scene = super::super::garage_scene::GarageScene::new(self.dir.clone(), host.screenshot.is_none())
            .with_rig(self.camera.clone());
        if !self.replace_scene(host, Box::new(scene), true) {
            return;
        }
        self.screens.clear();
        self.stage = Stage::Menus;
        self.latch = true;
        self.menu_garage = true;
        self.menu_garage_requested = true;
    }

    pub(in crate::frontend) fn poll_garage(&mut self, host: &mut Host, env: &mut Env, overlay: &mut Overlay) {
        if let Some(ui) = &self.garage_ui {
            if let Some(model) = host.scene.garage() {
                ui.sync(model);
            }
            overlay.menu = true;
            return;
        }
        if !self.menu_garage_requested && !overlay.menu_requested {
            overlay.menu = false;
            return;
        }
        overlay.menu = true;
        if !host.scene.ready() {
            host.hold_capture = true;
            return;
        }
        self.menu_garage_requested = false;
        overlay.menu_requested = false;
        let Some(renderer) = host.renderer.as_mut() else { return };
        if let Err(error) = host.scene.begin_garage(renderer.as_mut()) {
            log::error!("Cannot open garage: {error:#}");
            self.close_garage(host, env, overlay);
            let alert = super::super::confirm::Alert::new(crate::ui::Text::Custom(format!("{error:#}")));
            let commands = self.screens.open_with(
                super::super::ids::screen::DIALOG,
                Box::new(super::super::confirm::AlertLogic::new(alert)),
                false,
                env,
            );
            self.apply(commands, host, env);
            return;
        }
        let Some(model) = host.scene.garage() else { return };
        let page = Page::root(model);
        let ui = GarageUi::new();
        ui.sync(model);
        self.screens.clear();
        let commands = self.screens.open_with(page.package(), ui.logic(page), false, env);
        self.garage_ui = Some(ui);
        self.stage = Stage::Menus;
        self.latch = true;
        self.apply(commands, host, env);
        self.garage_requests(host, env, overlay);
    }

    pub(in crate::frontend) fn garage_requests(&mut self, host: &mut Host, env: &mut Env, overlay: &mut Overlay) {
        let Some(ui) = self.garage_ui.clone() else { return };
        for request in ui.drain() {
            match request {
                Request::Push(page) | Request::Replace(page) => {
                    let replace = matches!(request, Request::Replace(_));
                    let commands = self.screens.open_with(page.package(), ui.logic(page), replace, env);
                    self.apply(commands, host, env);
                }
                Request::Pop => {
                    let commands = self.screens.pop(env);
                    self.apply(commands, host, env);
                }
                Request::View(stage, camera) => {
                    if self.garage_tuning {
                        continue;
                    }
                    let stage = match (stage, host.scene.garage().is_some_and(|model| model.backroom)) {
                        (crate::garage::View::Customization, true) => crate::garage::View::Backroom,
                        _ => stage,
                    };
                    let Some(renderer) = host.renderer.as_mut() else { continue };
                    if let Err(error) = host.scene.garage_view(renderer.as_mut(), stage, camera) {
                        self.garage_error(host, error);
                    }
                }
                Request::Backroom(backroom) => {
                    let selection = host.scene.garage().map(|model| {
                        model.backroom = backroom;
                        model.draft.clone()
                    });
                    if let Some(selection) = selection {
                        self.garage_event(MenuEvent::Preview(selection), host);
                    }
                }
                Request::Save => {
                    if !self.save_garage(host) {
                        continue;
                    }
                    if let Some(model) = host.scene.garage() {
                        ui.saved(model);
                    }
                    self.garage_main(host, env, &ui);
                }
                Request::Main => self.garage_main(host, env, &ui),
                Request::SaveTuning { resume } => {
                    let selection = host.scene.garage().map(|model| model.draft.clone());
                    let Some(renderer) = host.renderer.as_mut() else { continue };
                    let Some(selection) = selection else { continue };
                    if let Err(error) = host.scene.confirm_tuning(renderer.as_mut(), selection) {
                        self.garage_error(host, error);
                        if let Some(model) = host.scene.garage() {
                            ui.sync(model);
                        }
                        let page = Page::TuningError;
                        let commands = self.screens.open_with(page.package(), ui.logic(page), false, env);
                        self.apply(commands, host, env);
                        continue;
                    }
                    self.close_garage(host, env, overlay);
                    if resume {
                        self.resume();
                    }
                    break;
                }
                Request::Event(event) => {
                    let event = *event;
                    let close = matches!(event, MenuEvent::Confirm(_) | MenuEvent::Cancel);
                    let slot = matches!(event, MenuEvent::SelectSlot(_) | MenuEvent::AddSlot | MenuEvent::DeleteSlot);
                    let success = self.garage_event(event, host);
                    if !success && slot {
                        // A following page transition belongs to the successful slot transaction.
                        break;
                    }
                    if success
                        && slot
                        && let Some(model) = host.scene.garage()
                    {
                        ui.saved(model);
                    }
                    if success && close {
                        self.close_garage(host, env, overlay);
                        break;
                    }
                }
            }
            if let Some(model) = host.scene.garage() {
                ui.sync(model);
            }
        }
    }

    fn save_garage(&mut self, host: &mut Host) -> bool {
        let Some(model) = host.scene.garage() else { return false };
        let selection = model.draft.clone();
        if !self.garage_event(MenuEvent::Confirm(selection), host) {
            return false;
        }
        let Some(renderer) = host.renderer.as_mut() else { return false };
        if let Err(error) = host.scene.begin_garage(renderer.as_mut()) {
            self.garage_error(host, error);
            return false;
        }
        true
    }

    fn garage_main(&mut self, host: &mut Host, env: &mut Env, ui: &GarageUi) {
        let slots = host.scene.garage().is_some_and(|model| model.custom_slots_enabled);
        self.screens.clear();
        if slots {
            let commands = self.screens.open_with(Page::Manager.package(), ui.logic(Page::Manager), false, env);
            self.apply(commands, host, env);
        }
        let commands = self.screens.open_with(Page::Main.package(), ui.logic(Page::Main), false, env);
        self.apply(commands, host, env);
    }

    fn garage_event(&mut self, event: MenuEvent, host: &mut Host) -> bool {
        if self.garage_tuning
            && let MenuEvent::Preview(selection) = &event
        {
            if let Some(model) = host.scene.garage() {
                model.draft = selection.clone();
            }
            return true;
        }
        let Some(renderer) = host.renderer.as_mut() else { return false };
        let result = match event {
            MenuEvent::Preview(selection) => host.scene.preview_garage(renderer.as_mut(), selection),
            MenuEvent::Confirm(selection) => host.scene.confirm_garage(renderer.as_mut(), selection),
            MenuEvent::Cancel => host.scene.cancel_garage(renderer.as_mut()),
            MenuEvent::SelectSlot(id) => host.scene.select_garage_slot(renderer.as_mut(), &id),
            MenuEvent::AddSlot => host.scene.add_garage_slot(renderer.as_mut()),
            MenuEvent::DeleteSlot => host.scene.delete_garage_slot(renderer.as_mut()),
        };
        if let Err(error) = result {
            self.garage_error(host, error);
            return false;
        }
        if let Some(model) = host.scene.garage() {
            model.message = None;
        }
        true
    }

    fn garage_error(&self, host: &mut Host, error: anyhow::Error) {
        log::warn!("Garage transaction: {error:#}");
        if let Some(model) = host.scene.garage() {
            model.message = Some(Err(format!("{error:#}")));
            if let Some(ui) = &self.garage_ui {
                ui.failed(model);
            }
        }
    }

    pub(in crate::frontend) fn close_garage(&mut self, host: &mut Host, env: &mut Env, overlay: &mut Overlay) {
        self.garage_ui = None;
        self.menu_garage_requested = false;
        overlay.menu = false;
        overlay.menu_requested = false;
        if std::mem::take(&mut self.garage_tuning) {
            while self.screens.top().is_some_and(|name| name != super::super::ids::screen::PAUSE_MENU) {
                let commands = self.screens.pop(env);
                self.apply(commands, host, env);
            }
            self.latch = true;
            return;
        }
        if std::mem::take(&mut self.menu_garage) {
            self.show_menu(host, env);
            return;
        }
        self.resume();
    }
}

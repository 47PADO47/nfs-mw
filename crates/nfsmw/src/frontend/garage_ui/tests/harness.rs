//! Installed-package input harness; transactions are observed without writing saves.

use super::super::{GarageUi, Page, Request};
use crate::frontend::{logic::Env, screens::Screens};
use crate::garage::{Car, Category, MenuEvent, Model, Selection};
use crate::settings::{Partial, Settings, test_caps};
use crate::ui::{Catalog, SCREEN_FILES, UiAssets};
use game_install::GameDir;
use nfsmw_data::car::customization::{Catalog as Parts, Category as Part, Choice};
use std::sync::{Arc, OnceLock};

pub(super) struct Harness {
    pub screens: Screens,
    pub ui: GarageUi,
    pub model: Model,
    pub events: Vec<MenuEvent>,
    pub saved: Vec<Selection>,
    pub fail_save: bool,
    pub fail_slot: bool,
    pub slot_selections: std::collections::HashMap<String, Selection>,
    settings: Settings,
    changed: Partial,
}

fn install() -> GameDir {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("installed garage UI test requires NFSMW_GAME_DIR");
    GameDir::open(std::path::PathBuf::from(root)).unwrap()
}

pub(super) fn model() -> Model {
    let mut parts = Parts::default();
    for category in Part::MODELS {
        parts.choices[category as usize].push(Choice {
            hash: category as u32 + 100,
            label: format!("{} 1", category.label()),
            language_hash: None,
            carbon: false,
        });
    }
    let mut car = Car {
        folder: "BMWM3GTR".into(),
        name: "BMW M3 GTR".into(),
        manufacturer: "BMW".into(),
        category: Category::Racing,
        paintable: true,
        parts: Some(parts.into()),
        ..Car::default()
    };
    for option in &mut car.performance.categories {
        option.max_level = 2;
    }
    let selection = Selection { car: car.folder.clone(), ..Selection::default() };
    Model { cars: vec![car], draft: selection.clone(), applied: selection, custom_slot_limit: 20, ..Model::default() }
}

impl Harness {
    pub fn open(page: Page, model: Model) -> Self {
        static ASSETS: OnceLock<Arc<UiAssets>> = OnceLock::new();
        let dir = install();
        let assets = ASSETS.get_or_init(|| Arc::new(UiAssets::load(&dir).unwrap())).clone();
        let catalog = Catalog::load(&dir, &SCREEN_FILES);
        assert!(catalog.find(page.package()).is_some(), "missing original {}", page.package());
        let ui = GarageUi::new();
        ui.sync(&model);
        let slot_selections = model.custom_slot.iter().map(|id| (id.clone(), model.draft.clone())).collect();
        let mut h = Self {
            screens: Screens::new(catalog, assets),
            ui,
            model,
            events: Vec::new(),
            saved: Vec::new(),
            fail_save: false,
            fail_slot: false,
            slot_selections,
            settings: Settings::from(Partial::default()),
            changed: Partial::default(),
        };
        h.open_page(page, false);
        h.run(0, 120);
        h
    }

    fn env<'a>(settings: &'a mut Settings, changed: &'a mut Partial) -> Env<'a> {
        Env { settings, changed, caps: test_caps::native(), mods: None }
    }

    fn open_page(&mut self, page: Page, replace: bool) {
        let logic = self.ui.logic(page);
        let mut env = Self::env(&mut self.settings, &mut self.changed);
        self.screens.open_with(page.package(), logic, replace, &mut env);
    }

    pub fn flush(&mut self) {
        for _ in 0..32 {
            let requests = self.ui.drain();
            if requests.is_empty() {
                return;
            }
            for request in requests {
                match request {
                    Request::Push(page) => self.open_page(page, false),
                    Request::Replace(page) => self.open_page(page, true),
                    Request::Pop => {
                        let mut env = Self::env(&mut self.settings, &mut self.changed);
                        self.screens.pop(&mut env);
                    }
                    Request::Event(event) => {
                        let event = *event;
                        if self.fail_slot
                            && matches!(event, MenuEvent::AddSlot | MenuEvent::DeleteSlot | MenuEvent::SelectSlot(_))
                        {
                            self.model.message = Some(Err("synthetic slot write failure".into()));
                            self.ui.failed(&self.model);
                            return;
                        }
                        match &event {
                            MenuEvent::Preview(selection) => {
                                if selection.car != self.model.draft.car {
                                    self.model.custom_slot = None;
                                }
                                self.model.draft = selection.clone();
                            }
                            MenuEvent::SelectSlot(id) => {
                                if let Some(selection) = self.slot_selections.get(id) {
                                    self.model.draft = selection.clone();
                                    self.model.applied = selection.clone();
                                }
                                self.model.custom_slot = Some(id.clone());
                                self.ui.saved(&self.model);
                            }
                            MenuEvent::AddSlot => {
                                let id = format!("slot{}", self.model.custom_slots.len());
                                self.model.custom_slots.push((id.clone(), "BMW M3 GTR".into()));
                                self.model.custom_slot_cars.insert(id.clone(), self.model.draft.car.clone());
                                self.slot_selections.insert(id.clone(), self.model.draft.clone());
                                self.model.custom_slot = Some(id);
                                self.model.applied = self.model.draft.clone();
                                self.ui.saved(&self.model);
                            }
                            MenuEvent::DeleteSlot => {
                                if let Some(id) = &self.model.custom_slot {
                                    self.slot_selections.remove(id);
                                }
                                self.model.custom_slots.retain(|(id, _)| Some(id) != self.model.custom_slot.as_ref());
                                self.model.custom_slot = None;
                                self.ui.saved(&self.model);
                            }
                            _ => {}
                        }
                        self.events.push(event);
                        self.ui.sync(&self.model);
                    }
                    Request::Backroom(backroom) => {
                        self.model.backroom = backroom;
                        self.ui.sync(&self.model);
                    }
                    Request::View(..) => {}
                    Request::Main => {
                        self.screens.clear();
                        if self.model.custom_slots_enabled {
                            self.open_page(Page::Manager, false);
                        }
                        self.open_page(Page::Main, false);
                    }
                    request @ (Request::Save | Request::SaveTuning { .. }) => {
                        if self.fail_save {
                            self.model.message = Some(Err("synthetic save failure".into()));
                            self.ui.failed(&self.model);
                            if matches!(request, Request::SaveTuning { .. }) {
                                self.open_page(Page::TuningError, false);
                            }
                            continue;
                        }
                        self.saved.push(self.model.draft.clone());
                        self.ui.saved(&self.model);
                        self.ui.request(Request::Main);
                    }
                }
            }
        }
        panic!("garage UI request cycle");
    }

    pub fn run(&mut self, mask: u32, frames: usize) {
        for _ in 0..frames {
            let mut env = Self::env(&mut self.settings, &mut self.changed);
            self.screens.update(1.0 / 60.0, mask, &mut env);
            self.flush();
        }
    }

    pub fn press(&mut self, mask: u32) {
        self.run(mask, 3);
        self.run(0, 90);
    }

    pub fn click_named(&mut self, name: u32) {
        let point = self
            .screens
            .presented(&crate::input::Bindings::default(), crate::input::InputDevice::Keyboard)
            .last()
            .unwrap()
            .tree
            .nodes
            .iter()
            .find(|node| node.name_hash == name && node.visible)
            .unwrap()
            .world
            .transform_point3(glam::Vec3::ZERO)
            .truncate();
        self.click_point(point);
    }

    pub fn click_point(&mut self, point: glam::Vec2) {
        let mut env = Self::env(&mut self.settings, &mut self.changed);
        let commands = self.screens.click(point, &mut env).expect("native control must take its pointer click");
        assert!(commands.is_empty());
        self.flush();
        self.run(0, 90);
    }

    pub fn top(&self) -> &str {
        self.screens.top().unwrap_or("")
    }
}

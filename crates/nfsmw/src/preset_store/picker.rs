//! Named preset controls for the garage and testing overlays.
use std::path::PathBuf;

#[derive(Clone, Default)]
pub struct Picker {
    pub(crate) path: Option<PathBuf>,
    pub(crate) name: String,
    names: Vec<String>,
    filter: String,
    selected: String,
    manage: bool,
    initialized: bool,
    message: Option<Result<String, String>>,
}

pub enum Action {
    Save,
    Load,
}

impl Picker {
    /// Visible one-click rows for rapidly comparing car setups.
    pub fn show_quick(&mut self, ui: &mut egui::Ui, filename: &str) -> Option<Action> {
        if !self.initialized {
            self.initialized = true;
            if self.path.is_none() {
                self.path = crate::settings::Settings::config_path().map(|p| p.with_file_name(filename));
            }
            self.refresh();
        }
        let mut action = None;
        ui.label("Saved presets · click to load");
        ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Search saved presets"));
        egui::ScrollArea::vertical().id_salt((filename, "quick")).max_height(125.0).show(ui, |ui| {
            let filter = self.filter.to_lowercase();
            for name in self.names.clone().into_iter().filter(|n| n.to_lowercase().contains(&filter)) {
                if ui.selectable_label(self.selected == name, &name).clicked() {
                    self.selected = name.clone();
                    self.name = name;
                    action = Some(Action::Load);
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.add_enabled(!self.selected.is_empty(), egui::Button::new("Update selected")).clicked() {
                self.name = self.selected.clone();
                action = Some(Action::Save);
            }
            ui.toggle_value(&mut self.manage, "Save as / manage…");
        });
        if self.manage {
            ui.add(egui::TextEdit::singleline(&mut self.name).char_limit(64).hint_text("Preset name"));
            ui.horizontal_wrapped(|ui| {
                if ui.button("Save as").clicked() {
                    action = Some(Action::Save);
                }
                if ui.button("Rename selected").clicked() {
                    let result = self
                        .path
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("No preset folder"))
                        .and_then(|p| super::rename(p, &self.selected, &self.name))
                        .map(|_| "Preset renamed".into());
                    if result.is_ok() {
                        self.selected = self.name.trim().into();
                    }
                    self.report(result);
                    self.refresh();
                }
                if ui.button("Delete selected").clicked() {
                    let result = self
                        .path
                        .as_deref()
                        .ok_or_else(|| anyhow::anyhow!("No preset folder"))
                        .and_then(|p| super::delete(p, &self.selected))
                        .map(|_| "Preset deleted".into());
                    if result.is_ok() {
                        self.selected.clear();
                    }
                    self.report(result);
                    self.refresh();
                }
                if ui.button("Refresh").clicked() {
                    self.refresh();
                }
            });
        }
        if let Some(message) = &self.message {
            match message {
                Ok(text) => {
                    ui.small(text);
                }
                Err(text) => {
                    ui.colored_label(egui::Color32::LIGHT_RED, text);
                }
            }
        }
        action
    }

    fn refresh(&mut self) {
        let result =
            self.path.as_deref().ok_or_else(|| anyhow::anyhow!("No user config folder")).and_then(super::names);
        match result {
            Ok(names) => self.names = names,
            Err(error) => self.report(Err(error)),
        }
    }

    pub fn save(&mut self, payload: anyhow::Result<String>) {
        let result = payload.and_then(|payload| {
            let path = self.path.as_deref().ok_or_else(|| anyhow::anyhow!("No user config folder"))?;
            super::save(path, &self.name, &payload)?;
            Ok(format!("Preset '{}' saved.", self.name.trim()))
        });
        if result.is_ok() {
            self.name = self.name.trim().to_owned();
            self.selected = self.name.clone();
        }
        self.refresh();
        self.report(result);
    }

    pub fn load(&self) -> anyhow::Result<String> {
        let path = self.path.as_deref().ok_or_else(|| anyhow::anyhow!("No user config folder"))?;
        super::load(path, &self.name)
    }

    pub fn report(&mut self, result: anyhow::Result<String>) {
        self.message = Some(result.map_err(|e| format!("{e:#}")));
    }
}

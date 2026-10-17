//! The same named-preset controls in F6 and F8.
use std::path::PathBuf;

#[derive(Default)]
pub struct Picker {
    pub(crate) path: Option<PathBuf>,
    pub(crate) name: String,
    names: Vec<String>,
    initialized: bool,
    message: Option<Result<String, String>>,
}

pub enum Action {
    Save,
    Load,
}

impl Picker {
    pub fn show(&mut self, ui: &mut egui::Ui, filename: &str) -> Option<Action> {
        if !self.initialized {
            self.initialized = true;
            if self.path.is_none() {
                self.path = crate::settings::Settings::config_path().map(|p| p.with_file_name(filename));
            }
            self.refresh();
        }
        let mut action = None;
        ui.collapsing("Saved presets", |ui| {
            egui::ComboBox::from_id_salt(filename).selected_text("Choose saved preset").show_ui(ui, |ui| {
                for name in &self.names {
                    ui.selectable_value(&mut self.name, name.clone(), name);
                }
            });
            ui.add(egui::TextEdit::singleline(&mut self.name).char_limit(64).hint_text("Preset name"));
            ui.horizontal_wrapped(|ui| {
                if ui.button("Save preset").on_hover_text("Saves or replaces this named preset.").clicked() {
                    action = Some(Action::Save);
                }
                if ui.add_enabled(self.names.contains(&self.name), egui::Button::new("Load preset")).clicked() {
                    action = Some(Action::Load);
                }
                if ui.button("Refresh").clicked() {
                    self.refresh();
                }
            });
            ui.small("Named presets persist across launches. Loading previews immediately.");
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
        });
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

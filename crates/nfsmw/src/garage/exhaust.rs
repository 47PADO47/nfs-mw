//! Local enhanced-exhaust appearance, stored with an F6 car preset.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExhaustSelection {
    pub root: [u8; 3],
    pub tail: [u8; 3],
    #[serde(skip_serializing)]
    pub length: u16,
    #[serde(skip_serializing)]
    pub width: u16,
    #[serde(skip_serializing)]
    pub brightness: u16,
    #[serde(skip_serializing)]
    pub density: u16,
    #[serde(skip_serializing)]
    pub flow: u16,
    #[serde(skip_serializing)]
    pub turbulence: u16,
    #[serde(skip_serializing)]
    pub curl: u16,
    #[serde(skip_serializing)]
    pub response: u16,
}

impl Default for ExhaustSelection {
    fn default() -> Self {
        let style = crate::settings::ExhaustStyle::default();
        Self {
            root: [80, 160, 255],
            tail: [212, 175, 140],
            length: style.length,
            width: style.width,
            brightness: style.brightness,
            density: style.density,
            flow: style.flow,
            turbulence: style.turbulence,
            curl: style.curl,
            response: style.response,
        }
    }
}

impl ExhaustSelection {
    /// Resolve release-relative percentages to the flame's original authoring units.
    /// A value of 100 reproduces the approved release appearance for each control.
    pub fn authored(&self) -> Self {
        let scale = |value: u16, baseline: u16| (f32::from(value) * f32::from(baseline) / 100.0).round() as u16;
        Self {
            length: scale(self.length, 64),
            width: scale(self.width, 62),
            brightness: scale(self.brightness, 150),
            density: scale(self.density, 137),
            flow: scale(self.flow, 155),
            turbulence: scale(self.turbulence, 45),
            curl: scale(self.curl, 27),
            response: scale(self.response, 200),
            ..self.clone()
        }
    }

    pub fn style(&self) -> crate::settings::ExhaustStyle {
        crate::settings::ExhaustStyle {
            length: self.length,
            width: self.width,
            brightness: self.brightness,
            density: self.density,
            flow: self.flow,
            turbulence: self.turbulence,
            curl: self.curl,
            response: self.response,
        }
    }

    pub fn set_style(&mut self, style: crate::settings::ExhaustStyle) {
        self.length = style.length;
        self.width = style.width;
        self.brightness = style.brightness;
        self.density = style.density;
        self.flow = style.flow;
        self.turbulence = style.turbulence;
        self.curl = style.curl;
        self.response = style.response;
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        self.style().validate()
    }

    pub fn compact() -> Self {
        Self { length: 86, width: 97, brightness: 93, ..Self::default() }
    }

    pub fn aggressive() -> Self {
        Self { length: 156, width: 137, brightness: 117, density: 102, flow: 103, turbulence: 111, ..Self::default() }
    }
}

pub(super) fn show(ui: &mut egui::Ui, value: &mut ExhaustSelection, enabled: bool, preview: &mut bool) {
    ui.collapsing("Exhaust", |ui| {
        if !enabled {
            ui.weak("Enable Enhanced exhaust in F8 to see these settings.");
        }
        ui.add_enabled_ui(enabled, |ui| {
            ui.checkbox(preview, "Preview flame");
        });
        let presets = [
            ("Default", ExhaustSelection::default()),
            ("Compact", ExhaustSelection::compact()),
            ("Aggressive", ExhaustSelection::aggressive()),
        ];
        let label = presets.iter().find(|(_, preset)| preset == value).map_or("Custom", |(name, _)| *name);
        egui::ComboBox::from_id_salt("exhaust_preset").selected_text(label).show_ui(ui, |ui| {
            for (name, preset) in presets {
                if ui.selectable_label(*value == preset, name).clicked() {
                    *value = preset;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Root tint");
            ui.color_edit_button_srgb(&mut value.root);
        });
        ui.horizontal(|ui| {
            ui.label("Tail tint");
            ui.color_edit_button_srgb(&mut value.tail);
        });
        for (label, field) in [
            ("Length", &mut value.length),
            ("Width", &mut value.width),
            ("Brightness", &mut value.brightness),
            ("Density", &mut value.density),
        ] {
            ui.add(egui::Slider::new(field, 0..=200).text(label).suffix("%"));
        }
        ui.collapsing("Motion", |ui| {
            for (label, field) in [
                ("Flow speed", &mut value.flow),
                ("Turbulence", &mut value.turbulence),
                ("Upward curl", &mut value.curl),
                ("Motion response", &mut value.response),
            ] {
                ui.add(egui::Slider::new(field, 0..=200).text(label).suffix("%"));
            }
        });
        if ui.button("Reset exhaust").clicked() {
            *value = ExhaustSelection::default();
        }
        ui.weak("Shape and motion are shared across cars and saved automatically. Colours stay with the car preset.");
        ui.weak("Heat, airflow and motion shape one curve. The parked preview has no driving forces.");
    });
}

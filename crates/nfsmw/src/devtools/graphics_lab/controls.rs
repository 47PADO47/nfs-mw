//! Small resets use the selected style and quality, rather than stale startup values.
pub(super) fn reset<T: Copy + PartialEq>(ui: &mut egui::Ui, value: &mut T, default: T, label: &str) {
    if ui
        .add_enabled(*value != default, egui::Button::new("↺").small())
        .on_hover_text(format!("Reset {label} to the selected style/quality default"))
        .clicked()
    {
        *value = default;
    }
}

pub(super) fn checkbox(ui: &mut egui::Ui, value: &mut bool, label: &str, default: bool) -> egui::Response {
    ui.horizontal(|ui| {
        let response = ui.checkbox(value, label);
        reset(ui, value, default, label);
        response
    })
    .inner
}

pub(super) fn slider(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    label: &str,
    default: f32,
) -> egui::Response {
    ui.horizontal(|ui| {
        let response = ui.add(egui::Slider::new(value, range).text(label));
        reset(ui, value, default, label);
        response
    })
    .inner
}

pub(super) fn choice<T: Copy + PartialEq + std::fmt::Display>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    choices: &[T],
    default: T,
) {
    ui.horizontal(|ui| {
        super::view::choice(ui, label, value, choices);
        reset(ui, value, default, label);
    });
}

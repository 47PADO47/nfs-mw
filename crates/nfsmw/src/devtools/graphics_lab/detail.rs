//! Main-view detail; other render views keep their own budgets.
use crate::settings::{CarGeometry, DetailSettings, Settings, WorldDetail, detail::DetailRange};

pub(super) fn show(ui: &mut egui::Ui, settings: &mut Settings) {
    let detail = &mut settings.detail;
    select(ui, "World Level of Detail", &mut detail.world, &WorldDetail::ALL, WorldDetail::label);
    ui.small("Original four-level mesh policy. Texture quality changes on the next world load; save and restart to apply it consistently.");
    select(ui, "Car Geometry Detail", &mut detail.car, &CarGeometry::ALL, CarGeometry::label);
    ui.checkbox(&mut detail.highest_player, "Keep player car at highest detail");
    ui.separator();
    ui.label("Extended world detail");
    ui.horizontal_wrapped(|ui| {
        for (name, range) in [
            ("Original ranges", DetailRange::Original),
            ("Extended 1.5×", DetailRange::Extended),
            ("Extended 2×", DetailRange::Double),
        ] {
            if ui.button(name).clicked() {
                detail.geometry_range = range;
                detail.visibility_range = range;
                detail.highest_world = false;
            }
        }
    });
    select(ui, "Detailed mesh range", &mut detail.geometry_range, &DetailRange::ALL, DetailRange::label);
    select(ui, "Small-object visibility", &mut detail.visibility_range, &DetailRange::ALL, DetailRange::label);
    ui.add_enabled_ui(!detail.world.coarse(), |ui| {
        ui.checkbox(&mut detail.highest_world, "Highest visible world meshes (diagnostic)");
    });
    if detail.highest_world && !detail.world.coarse() {
        ui.small("Diagnostic: coarse-only scenery can disappear when its detailed slot is absent.");
    }
    ui.small("Extensions affect the main view within authored map sections. Shadows, reflections and distant section coverage keep independent budgets.");
    ui.small("Original projection calibration and some per-usage car rules remain incomplete; these controls do not imply exact visual parity.");
    if ui.button("Reset detail").clicked() {
        *detail = DetailSettings::default();
    }
}

fn select<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    choices: &[T],
    name: fn(T) -> &'static str,
) {
    egui::ComboBox::from_label(label).selected_text(name(*value)).show_ui(ui, |ui| {
        for &choice in choices {
            ui.selectable_value(value, choice, name(choice));
        }
    });
}

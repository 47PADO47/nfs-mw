//! Main-view detail; other render views keep their own budgets.
use crate::settings::{CarGeometry, Settings, WorldDetail, detail::DetailRange};

pub(super) fn show(ui: &mut egui::Ui, settings: &mut Settings) {
    let defaults = super::profiles::defaults(settings).detail;
    let detail = &mut settings.detail;
    select(ui, "World Level of Detail", &mut detail.world, &WorldDetail::ALL, WorldDetail::label, defaults.world);
    ui.small("Original four-level mesh policy. Texture quality changes on the next world load; save and restart to apply it consistently.");
    select(ui, "Car Geometry Detail", &mut detail.car, &CarGeometry::ALL, CarGeometry::label, defaults.car);
    super::controls::checkbox(
        ui,
        &mut detail.highest_player,
        "Keep player car at highest detail",
        defaults.highest_player,
    );
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
        if ui.button("Full scenery").clicked() {
            detail.world = WorldDetail::Maximum;
            detail.highest_world = true;
        }
    });
    ui.add_enabled_ui(!detail.highest_world || detail.world.coarse(), |ui| {
        select(
            ui,
            "Detailed mesh range",
            &mut detail.geometry_range,
            &DetailRange::ALL,
            DetailRange::label,
            defaults.geometry_range,
        );
        select(
            ui,
            "Small-object visibility",
            &mut detail.visibility_range,
            &DetailRange::ALL,
            DetailRange::label,
            defaults.visibility_range,
        );
    });
    ui.add_enabled_ui(!detail.world.coarse(), |ui| {
        super::controls::checkbox(
            ui,
            &mut detail.highest_world,
            "Keep scenery at highest detail",
            defaults.highest_world,
        );
    });
    if detail.highest_world && !detail.world.coarse() {
        ui.small("Full scenery disables mesh switching and small-object cutoffs. Coarse-only objects retain their authored mesh.");
    }
    ui.small(
        "Car detail changes do not control traffic spawning. Whole-car appearances use the separate population rules.",
    );
    ui.small("Extensions affect the main view within authored map sections. Shadows, reflections and distant section coverage keep independent budgets.");
    ui.small("Original projection calibration and some per-usage car rules remain incomplete; these controls do not imply exact visual parity.");
}

fn select<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    choices: &[T],
    name: fn(T) -> &'static str,
    default: T,
) {
    ui.horizontal(|ui| {
        egui::ComboBox::from_label(label).selected_text(name(*value)).show_ui(ui, |ui| {
            for &choice in choices {
                ui.selectable_value(value, choice, name(choice));
            }
        });
        super::controls::reset(ui, value, default, label);
    });
}

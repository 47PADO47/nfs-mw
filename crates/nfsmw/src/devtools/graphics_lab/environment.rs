//! Session controls over the existing world clock and forced rain model.
use super::*;

pub(super) fn show(ui: &mut egui::Ui, lab: &mut GraphicsLab, settings: &mut Settings) {
    ui.separator();
    ui.heading("Time & weather");
    let Some(mut value) = lab.environment else {
        ui.weak("Enter the world to change time and weather.");
        return;
    };
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        for (label, hour) in [("Dawn", 6.0), ("Morning", 9.0), ("Noon", 12.0), ("Dusk", 18.0), ("Night", 22.0)] {
            if ui.button(label).clicked() {
                value.hour = hour;
                changed = true;
            }
        }
    });
    changed |= ui
        .add(egui::Slider::new(&mut value.hour, 0.0..=23.99).text("Time").custom_formatter(|hour, _| {
            let minutes = (hour * 60.0).round() as u32 % 1440;
            format!("{:02}:{:02}", minutes / 60, minutes % 60)
        }))
        .changed();
    ui.checkbox(&mut lab.options.freeze_sun, "Freeze time");
    changed |= ui
        .add(
            egui::Slider::new(&mut value.seconds_per_hour, 10.0..=3600.0).logarithmic(true).text("Seconds / game hour"),
        )
        .changed();
    ui.small("Night uses dusk lighting. Sky and street lights are not yet time driven.");
    ui.horizontal(|ui| {
        ui.label("Weather:");
        ui.selectable_value(&mut settings.always_rain, false, "Clear");
        if ui.selectable_value(&mut settings.always_rain, true, "Rain").clicked() && settings.rain_amount.0 == 0 {
            settings.rain_amount.0 = 100;
        }
    });
    ui.add_enabled_ui(settings.always_rain, |ui| {
        ui.add(egui::Slider::new(&mut settings.rain_amount.0, 0..=100).text("Rain amount %"));
    });
    ui.small("Rain fades during simulation; regional storms and cloud changes are not implemented.");
    if changed {
        lab.environment = Some(value);
        lab.environment_change = Some(value);
    }
}

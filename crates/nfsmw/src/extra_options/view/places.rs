use super::*;
pub(super) fn show(ui: &mut egui::Ui, state: &mut State, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        if ui.button("Teleport to GPS destination").clicked() {
            actions.push(Action::Marker);
        }
        if ui.add_enabled(state.previous_location.is_some(), egui::Button::new("Return to previous position")).clicked()
        {
            actions.push(Action::Return);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Map X / Y:");
        ui.add(egui::DragValue::new(&mut state.coordinates[0]));
        ui.add(egui::DragValue::new(&mut state.coordinates[1]));
        if ui.button("Teleport to road").clicked() {
            actions.push(Action::Teleport(state.coordinates, None));
        }
    });
    ui.small("Teleport searches for a safe road and stops the car. End the current race first.");
    ui.separator();
    ui.add(egui::TextEdit::singleline(&mut state.name).char_limit(64).hint_text("Location / camera bookmark name"));
    if ui.button("Save current car location + heading").clicked() {
        actions.push(Action::SaveLocation);
    }
    for (index, v) in state.locations.iter().enumerate() {
        ui.horizontal(|ui| {
            if ui.button(&v.name).clicked() {
                actions.push(Action::Teleport(v.position, Some(v.heading)));
            }
            if ui.small_button("Delete").clicked() {
                actions.push(Action::DeleteLocation(index));
            }
        });
    }
    ui.separator();
    ui.checkbox(&mut state.relative, "Save view relative to car · follows car position and heading");
    if ui.button("Save current camera view").clicked() {
        actions.push(Action::SaveBookmark);
    }
    for (index, v) in state.bookmarks.iter().enumerate() {
        ui.horizontal(|ui| {
            let kind = match v.relative {
                true => "car",
                false => "world",
            };
            if ui.button(format!("{} ({kind})", v.name)).clicked() {
                actions.push(Action::LoadBookmark(index));
            }
            if ui.small_button("Delete").clicked() {
                actions.push(Action::DeleteBookmark(index));
            }
        });
    }
    if state.selected_bookmark.is_some() && ui.button("Release camera bookmark").clicked() {
        actions.push(Action::Center);
    }
}

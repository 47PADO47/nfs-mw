//! The F5 pursuit reference controls: a bounty place to jump to and the jump with or without a pursuit.
use super::{Action, State};
use crate::extra_options::reference;

pub(super) fn show(ui: &mut egui::Ui, state: &mut State, actions: &mut Vec<Action>) {
    ui.collapsing("Pursuit reference · original comparison", |ui| {
        spot_picker(ui, state, actions);
    });
}

fn spot_picker(ui: &mut egui::Ui, state: &mut State, actions: &mut Vec<Action>) {
    if state.bounty_spots.is_empty() {
        ui.weak("The install's bounty places are not loaded (no gameplay data).");
        return;
    }
    if state.bounty_selected.is_none() {
        state.bounty_selected = reference::default_spot(&state.bounty_spots);
    }
    let Some(selected) = state.bounty_selected else { return };
    egui::ComboBox::from_id_salt("bounty_spot")
        .width(320.0)
        .selected_text(reference::row_text(&state.bounty_spots, selected))
        .show_ui(ui, |ui| {
            for spot in &state.bounty_spots {
                let text = reference::row_text(&state.bounty_spots, spot.number);
                ui.selectable_value(&mut state.bounty_selected, Some(spot.number), text);
            }
        });
    ui.small("Bounty sheet places (Safehouse > Bounty). All 12 are offered here whatever the career stage.");
    ui.horizontal(|ui| {
        let police = !state.testing.police_off;
        if ui.add_enabled(police, egui::Button::new("Jump to spot + start pursuit")).clicked() {
            actions.push(Action::JumpToSpot { number: selected, pursuit: true });
        }
        if ui.button("Jump only").clicked() {
            actions.push(Action::JumpToSpot { number: selected, pursuit: false });
        }
    });
    let heat = state.testing.heat;
    ui.small(format!(
        "The car is put on the place's marker, facing its way and stopped, as the rap sheet does; the pursuit starts at testing heat {heat} once it stands there. Page Up / Page Down step the heat while driving."
    ));
    if let Some(note) = reference::heat_note(heat) {
        ui.small(format!("Heat {heat}: {note}."));
    }
    if let Some((number, _)) = state.pending_jump {
        ui.weak(format!("Waiting for the road at {}…", reference::row_text(&state.bounty_spots, number)));
    }
    if let Some((number, at, heading)) = state.landed {
        ui.weak(format!(
            "Last jump: {} at ({:.0}, {:.0}), heading {heading:.0}° (the place's marker Rotation)",
            reference::row_text(&state.bounty_spots, number),
            at[0],
            at[1]
        ));
    }
}

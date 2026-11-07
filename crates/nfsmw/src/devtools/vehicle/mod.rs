//! The vehicle panel (F11, or the devtools menu): every slot of the vehicle on show with its choices, the groups of optional
//! equipment, the body colour, and the rider's gear. What it lists comes from the vehicle's own data (`garage::options`), not from
//! a list of its own, and a change is on the vehicle at once, in the world and in the car viewer.
//!
//! The view is a pure function of what the vehicle offers: it returns what was asked, and `run_ui` applies it.

mod gear;
mod widgets;

use egui::{RichText, ScrollArea, vec2};

use crate::garage::options::{Change, Group, Slot, VehicleOptions, humanize};
use crate::garage::rider_gear::Part;
pub(crate) use gear::{GearRow, current as current_gear, list as gear_list, set as set_gear, stock as stock_gear};

/// What the panel asks for.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PanelAction {
    Vehicle(Change),
    /// Wear this item (its id; empty: the rider's own gear) on a part of the rider.
    Gear {
        part: Part,
        id: String,
    },
    Close,
}

/// What the panel shows.
pub(crate) struct PanelView<'a> {
    pub options: Option<&'a VehicleOptions>,
    pub gear: Option<&'a [GearRow]>,
    /// What the last change said.
    pub message: Option<&'a str>,
    /// A controller is connected: the first line takes the focus.
    pub pad: bool,
}

pub(crate) fn show(ctx: &egui::Context, view: &PanelView<'_>) -> Vec<PanelAction> {
    let mut actions = Vec::new();
    let max = (ctx.content_rect().height() - 70.0).max(300.0);
    egui::Window::new("Vehicle options · F11")
        .id(egui::Id::new("devtools_vehicle"))
        .default_pos(egui::pos2(24.0, 48.0))
        .default_size(vec2(460.0, 560.0))
        .max_height(max)
        .resizable(true)
        .show(ctx, |ui| {
            header(ui, view, &mut actions);
            ui.separator();
            let area = ScrollArea::vertical().auto_shrink([false, false]).max_height((max - 130.0).max(160.0));
            crate::devtools::pad_nav::scrolled(area, ui, |ui| body(ui, view, &mut actions));
            if let Some(message) = view.message {
                ui.separator();
                ui.label(RichText::new(message).weak());
            }
        });
    actions
}

fn header(ui: &mut egui::Ui, view: &PanelView<'_>, actions: &mut Vec<PanelAction>) {
    let Some(options) = view.options else {
        ui.label("No vehicle on show. Start driving (the drive command), or open a car in the car viewer.");
        if ui.button("Close").clicked() {
            actions.push(PanelAction::Close);
        }
        return;
    };
    let kind = match options.bike {
        true => "bike",
        false => "car",
    };
    ui.heading(&options.name);
    ui.weak(format!("{kind} · {} · changes apply at once", options.folder));
    ui.horizontal(|ui| {
        if ui.button("Reset to stock").clicked() {
            actions.push(PanelAction::Vehicle(Change::Reset));
        }
        if ui.button("Close").clicked() {
            actions.push(PanelAction::Close);
        }
    });
}

fn body(ui: &mut egui::Ui, view: &PanelView<'_>, actions: &mut Vec<PanelAction>) {
    let mut first = view.pad && ui.ctx().memory(|memory| memory.focused().is_none());
    if let Some(options) = view.options {
        let groups = options.groups();
        for group in &groups {
            group_section(ui, options, group, &mut first, actions);
        }
        colour(ui, options, actions);
        for line in &options.missing {
            ui.add_space(4.0);
            ui.label(RichText::new(format!("Not in this vehicle's data: {line}")).small().weak());
        }
    }
    if let Some(rows) = view.gear {
        gear_section(ui, rows, view.options.is_some_and(|options| options.bike), &mut first, actions);
    }
}

fn group_section(
    ui: &mut egui::Ui,
    options: &VehicleOptions,
    group: &Group,
    first: &mut bool,
    actions: &mut Vec<PanelAction>,
) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.heading(humanize(&group.name));
        if group.switchable {
            let mut on = group.on;
            if ui.checkbox(&mut on, "on").changed() {
                actions.push(PanelAction::Vehicle(Change::Group { name: group.name.clone(), on }));
            }
        }
        if group.changed > 0 && ui.small_button("stock").on_hover_text("Put the group back to stock").clicked() {
            actions.push(PanelAction::Vehicle(Change::ResetGroup(group.name.clone())));
        }
    });
    for slot in options.in_group(&group.name) {
        slot_row(ui, slot, first, actions);
    }
}

fn slot_row(ui: &mut egui::Ui, slot: &Slot, first: &mut bool, actions: &mut Vec<PanelAction>) {
    ui.label(RichText::new(&slot.label).small());
    let choice = slot.installed();
    let mut text = choice.label.clone();
    if let Some(price) = choice.price {
        text.push_str(&format!(" · ${price}"));
    }
    let id = ui.make_persistent_id(("vehicle_slot", &slot.id));
    let want_focus = std::mem::take(first);
    ui.horizontal(|ui| {
        let width = (ui.available_width() - 34.0).max(120.0);
        let asked = ui
            .scope(|ui| {
                ui.set_width(width);
                widgets::stepper(ui, id, &text, slot.current, slot.choices.len(), want_focus)
            })
            .inner;
        if let Some(index) = asked {
            actions.push(PanelAction::Vehicle(Change::Pick { slot: slot.id.clone(), index }));
        }
        if let Some(index) = list_button(ui, slot) {
            actions.push(PanelAction::Vehicle(Change::Pick { slot: slot.id.clone(), index }));
        }
    });
    if let Some(reason) = &choice.blocked {
        ui.label(RichText::new(reason).small().weak());
    }
}

/// The list of every choice of a slot, for a mouse: the stepper is for the controller and for short lists.
fn list_button(ui: &mut egui::Ui, slot: &Slot) -> Option<usize> {
    if slot.choices.len() < 3 {
        return None;
    }
    let mut picked = None;
    ui.menu_button("▾", |ui| {
        ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            for (at, choice) in slot.choices.iter().enumerate() {
                let label = match &choice.blocked {
                    Some(reason) => format!("{at}: {} ({reason})", choice.label),
                    None => format!("{at}: {}", choice.label),
                };
                if ui.selectable_label(at == slot.current, label).clicked() {
                    picked = Some(at);
                    ui.close();
                }
            }
        });
    });
    picked
}

fn colour(ui: &mut egui::Ui, options: &VehicleOptions, actions: &mut Vec<PanelAction>) {
    let Some(paint) = options.paintwork else { return };
    ui.add_space(6.0);
    ui.heading("Body colour");
    ui.horizontal(|ui| {
        let mut rgb = paint.custom.or(paint.original).unwrap_or([128; 3]);
        if ui.color_edit_button_srgb(&mut rgb).changed() {
            actions.push(PanelAction::Vehicle(Change::Color(Some(rgb))));
        }
        if paint.custom.is_none() {
            ui.weak("the vehicle's own paint");
        }
        if ui.add_enabled(paint.custom.is_some(), egui::Button::new("Own paint")).clicked() {
            actions.push(PanelAction::Vehicle(Change::Color(None)));
        }
    });
}

fn gear_section(ui: &mut egui::Ui, rows: &[GearRow], bike: bool, first: &mut bool, actions: &mut Vec<PanelAction>) {
    ui.add_space(6.0);
    let header =
        egui::CollapsingHeader::new(RichText::new("Rider gear").heading()).default_open(bike).id_salt("devtools_gear");
    header.show(ui, |ui| {
        ui.weak("Worn by the rider of a bike; kept in the profile like the garage's choice.");
        for row in rows {
            ui.label(RichText::new(row.part.label()).small());
            let id = ui.make_persistent_id(("gear_row", row.part.key()));
            let want_focus = std::mem::take(first) && bike;
            let value = &row.entries[row.worn].label;
            if let Some(at) = widgets::stepper(ui, id, value, row.worn, row.entries.len(), want_focus) {
                actions.push(PanelAction::Gear { part: row.part, id: row.entries[at].id.clone() });
            }
        }
        if ui.button("Rider's own gear").clicked() {
            actions.extend(rows.iter().map(|row| PanelAction::Gear { part: row.part, id: String::new() }));
        }
    });
}

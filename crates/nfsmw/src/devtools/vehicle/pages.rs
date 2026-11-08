//! The pages of the vehicle panel: what each lists comes from the vehicle's own data (`garage::options`).

use egui::RichText;
use nfsmw_data::car::customization::Category;

use super::{GearRow, Hosted, PanelAction, PanelView, widgets};
use crate::devtools::panels::VehiclePage;
use crate::garage::options::{Change, Group, Shortcut, Slot, Store, VehicleOptions, humanize};

/// The page a slot is listed on: the performance levels on Performance, the paint, tint, rim paint and vinyl on Appearance, the
/// rest (model parts, a vehicle's own component slots) on Parts.
pub(super) fn page_of(slot: &Slot) -> VehiclePage {
    match slot.store {
        Store::Level(_) => VehiclePage::Performance,
        Store::Category(Category::Paint | Category::WindowTint | Category::RimPaint | Category::Vinyl) => {
            VehiclePage::Appearance
        }
        Store::Category(_) | Store::Session => VehiclePage::Parts,
    }
}

pub(super) fn body(
    ui: &mut egui::Ui,
    view: &PanelView<'_>,
    page: VehiclePage,
    actions: &mut Vec<PanelAction>,
    hosted: &mut dyn FnMut(&mut egui::Ui, Hosted),
) {
    let mut first = view.pad && ui.ctx().memory(|memory| memory.focused().is_none());
    let bike = view.options.is_some_and(|options| options.bike);
    if page == VehiclePage::Gear {
        match view.gear {
            Some(rows) => gear(ui, rows, bike, &mut first, actions),
            None => {
                ui.weak("No rider gear: no mod offers RIDER/GEAR/gear.json.");
            }
        }
        return;
    }
    if page == VehiclePage::Tuning {
        match view.hosted {
            true => hosted(ui, Hosted::Page(page)),
            false => {
                ui.weak("The tuning profiles need the driving world: the car viewer has no physics.");
            }
        }
        return;
    }
    let Some(options) = view.options else { return };
    match page {
        VehiclePage::Appearance => finishes(ui, options, actions),
        VehiclePage::Performance => shortcuts(ui, options, actions),
        _ => {}
    }
    let mut listed = false;
    for group in options.groups() {
        listed |= group_section(ui, options, &group, page, &mut first, actions);
    }
    if !listed && page != VehiclePage::Appearance {
        ui.weak("This vehicle's data has nothing to change here.");
    }
    if page == VehiclePage::Appearance {
        colour(ui, options, actions);
        if view.hosted {
            hosted(ui, Hosted::Page(page));
        }
    }
    if page == VehiclePage::Parts {
        for line in &options.missing {
            ui.add_space(4.0);
            ui.label(RichText::new(format!("Not in this vehicle's data: {line}")).small().weak());
        }
    }
}

/// A group's slots on `page`, with the group's switch and its stock button. Whether the group has a slot on the page.
fn group_section(
    ui: &mut egui::Ui,
    options: &VehicleOptions,
    group: &Group,
    page: VehiclePage,
    first: &mut bool,
    actions: &mut Vec<PanelAction>,
) -> bool {
    let slots: Vec<&Slot> = options.in_group(&group.name).filter(|slot| page_of(slot) == page).collect();
    if slots.is_empty() {
        return false;
    }
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
    for slot in slots {
        slot_row(ui, slot, first, actions);
    }
    true
}

fn slot_row(ui: &mut egui::Ui, slot: &Slot, first: &mut bool, actions: &mut Vec<PanelAction>) {
    ui.label(RichText::new(&slot.label).small());
    let choice = slot.installed();
    let mut text = choice.label.clone();
    if let Some(price) = choice.price {
        text.push_str(&format!(" · ${price}"));
    }
    let id = ui.make_persistent_id(("vehicle_slot", &slot.id));
    let label_of = |at: usize| {
        let choice = &slot.choices[at];
        match &choice.blocked {
            Some(reason) => format!("{} ({reason})", choice.label),
            None => choice.label.clone(),
        }
    };
    let want_focus = std::mem::take(first);
    let asked = widgets::chooser(ui, id, &text, (slot.current, slot.choices.len(), 0), want_focus, &label_of);
    if let Some(index) = asked {
        actions.push(PanelAction::Vehicle(Change::Pick { slot: slot.id.clone(), index }));
    }
    if let Some(reason) = &choice.blocked {
        ui.label(RichText::new(reason).small().weak());
    }
    if let Some(junkman) = slot.junkman {
        let mut on = junkman.on;
        let response = ui.add_enabled(junkman.available, egui::Checkbox::new(&mut on, "Junkman package"));
        if response.changed() {
            actions.push(PanelAction::Vehicle(Change::Junkman { slot: slot.id.clone(), on }));
        }
    }
}

/// The performance shortcuts of the garage: stock, maximum, maximum with every Junkman package.
fn shortcuts(ui: &mut egui::Ui, options: &VehicleOptions, actions: &mut Vec<PanelAction>) {
    if options.levels.is_none() {
        ui.weak("The car viewer has no physics: no performance levels here.");
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for (label, shortcut) in
            [("Stock", Shortcut::Stock), ("Maximum", Shortcut::Maximum), ("Max + Junkman", Shortcut::MaximumJunkman)]
        {
            if ui.button(label).clicked() {
                actions.push(PanelAction::Vehicle(Change::Shortcut(shortcut)));
            }
        }
    });
    ui.weak("Each vehicle has the ladders its own data lists; they apply at once and keep the position and speed.");
}

/// The catalogue's quick finishes of the paint: the colour stays, the finish changes.
fn finishes(ui: &mut egui::Ui, options: &VehicleOptions, actions: &mut Vec<PanelAction>) {
    if options.finishes.is_empty() {
        return;
    }
    ui.label(RichText::new("Paint finish").small());
    ui.horizontal_wrapped(|ui| {
        for finish in &options.finishes {
            let response = ui
                .selectable_label(finish.active, &finish.label)
                .on_hover_text("Keep the current colour and change the finish");
            if response.clicked() {
                actions.push(PanelAction::Vehicle(Change::Finish(finish.hash)));
            }
        }
    });
}

fn colour(ui: &mut egui::Ui, options: &VehicleOptions, actions: &mut Vec<PanelAction>) {
    ui.add_space(6.0);
    ui.heading("Body colour");
    let Some(paint) = options.paintwork else {
        ui.weak("This vehicle has a fixed livery.");
        return;
    };
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

fn gear(ui: &mut egui::Ui, rows: &[GearRow], bike: bool, first: &mut bool, actions: &mut Vec<PanelAction>) {
    ui.weak("Worn by the rider of a bike; kept in the profile like the garage's choice.");
    if !bike {
        ui.weak("The vehicle on show is not a bike: the gear is worn on the next bike.");
    }
    for row in rows {
        ui.label(RichText::new(row.part.label()).small());
        let id = ui.make_persistent_id(("gear_row", row.part.key()));
        let want_focus = std::mem::take(first);
        let value = &row.entries[row.worn].label;
        let label_of = |at: usize| row.entries[at].label.clone();
        let asked = widgets::chooser(ui, id, value, (row.worn, row.entries.len(), 1), want_focus, &label_of);
        if let Some(at) = asked {
            actions.push(PanelAction::Gear { part: row.part, id: row.entries[at].id.clone() });
        }
    }
    if ui.button("Rider's own gear").clicked() {
        actions.extend(rows.iter().map(|row| PanelAction::Gear { part: row.part, id: String::new() }));
    }
}

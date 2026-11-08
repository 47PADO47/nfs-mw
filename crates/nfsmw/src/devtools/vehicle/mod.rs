//! The vehicle panel (F11, or the devtools menu): everything about the vehicle on show on pages: its parts, its appearance, its
//! performance ladders with the Junkman packages and the shortcuts, the tuning, and the rider's gear. What it lists comes from the vehicle's own data (`garage::options`), not from
//! a list of its own, and a change is on the vehicle at once, in the world and in the car viewer.
//!
//! The view is a pure function of what the vehicle offers: it returns what was asked, and `run_ui` applies it.

mod gear;
mod pages;
mod widgets;

use egui::{RichText, ScrollArea, vec2};

use super::panels::VehiclePage;
use crate::garage::options::{Change, VehicleOptions};
use crate::garage::rider_gear::Part;
pub(crate) use gear::{
    GearRow, current as current_gear, ensure_catalogue as ensure_gear_catalogue, list as gear_list, set as set_gear,
    stock as stock_gear,
};

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
    /// The scene has a garage: the pages that need it (tuning, the setups, the vinyl colours, the gauges, the plate) are hosted.
    pub hosted: bool,
}

/// What the garage hosts in the panel, beside the pages the vehicle's own data builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Hosted {
    /// The saved setups of the car: save, load, update.
    Setups,
    Page(VehiclePage),
}

pub(crate) fn show(
    ctx: &egui::Context,
    view: &PanelView<'_>,
    page: &mut VehiclePage,
    hosted: &mut dyn FnMut(&mut egui::Ui, Hosted),
) -> Vec<PanelAction> {
    let mut actions = Vec::new();
    let max = (ctx.content_rect().height() - 70.0).max(300.0);
    egui::Window::new("Vehicle options · F11")
        .id(egui::Id::new("devtools_vehicle"))
        .default_pos(egui::pos2(24.0, 48.0))
        .default_size(vec2(480.0, 600.0))
        .max_height(max)
        .resizable(true)
        .show(ctx, |ui| {
            header(ui, view, &mut actions);
            if view.hosted {
                hosted(ui, Hosted::Setups);
            }
            ui.horizontal_wrapped(|ui| {
                for tab in VehiclePage::ALL {
                    ui.selectable_value(page, tab, tab.label());
                }
            });
            ui.separator();
            let area = ScrollArea::vertical().auto_shrink([false, false]).max_height((max - 190.0).max(160.0));
            crate::devtools::pad_nav::scrolled(area, ui, |ui| pages::body(ui, view, *page, &mut actions, hosted));
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

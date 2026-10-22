//! The LAN menu and the session HUD drawn with egui, in the active UI theme's palette, radius, padding and text size (the
//! developer panels' style follows the theme: `devtools/theme_style.rs`). The original has no LAN screens, so there is no
//! FEng package to run; these panels are the stand-in (`docs/specs/multiplayer/frontend.md`).

use egui::{Align2, Color32, Frame, RichText, vec2};

use super::feed::FeedKind;
use super::hud::HudView;
use super::menu::LanKey;
use super::menu::rows::{PageView, Row, RowKind};

/// The menu panel's width and its rows' label column, points.
const PANEL_WIDTH: f32 = 760.0;
const LABEL_WIDTH: f32 = 300.0;
const SIDE_WIDTH: f32 = 240.0;

/// Draws `page` in the middle of the screen, its side column in a panel to the right, and returns the clicks as keys.
///
/// The rows and the side column are two windows, not two columns of one: `horizontal_top` with a `vertical` inside a
/// window that sizes itself crashed the renderer in a screenshot run (an access violation), so the layout stays flat.
pub fn show_page(ctx: &egui::Context, page: &PageView) -> Vec<LanKey> {
    let mut keys = Vec::new();
    let side = !page.side_title.is_empty();
    let offset = match side {
        true => -(SIDE_WIDTH + 24.0) / 2.0,
        false => 0.0,
    };
    egui::Window::new("lan_menu")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, vec2(offset, 0.0))
        .frame(Frame::window(&ctx.global_style()))
        .show(ctx, |ui| {
            ui.set_min_width(PANEL_WIDTH);
            ui.label(RichText::new(&page.title).heading().strong());
            ui.label(RichText::new(&page.subtitle).weak());
            if let Some(error) = &page.error {
                ui.colored_label(ui.visuals().warn_fg_color, error);
            }
            ui.separator();
            rows(ui, page, &mut keys);
            ui.separator();
            ui.horizontal(|ui| {
                for (key, what) in &page.hints {
                    ui.label(RichText::new(*key).strong());
                    ui.label(RichText::new(*what).weak());
                    ui.add_space(12.0);
                }
            });
        });
    if side {
        egui::Window::new("lan_side")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_CENTER, vec2((PANEL_WIDTH + 24.0) / 2.0, 0.0))
            .frame(Frame::window(&ctx.global_style()))
            .show(ctx, |ui| side_column(ui, page));
    }
    keys
}

fn rows(ui: &mut egui::Ui, page: &PageView, keys: &mut Vec<LanKey>) {
    egui::Grid::new("lan_rows").num_columns(2).min_col_width(LABEL_WIDTH).spacing(vec2(16.0, 6.0)).show(ui, |ui| {
        for (at, row) in page.rows.iter().enumerate() {
            let selected = at == page.selected;
            let mut label = RichText::new(&row.label);
            if selected {
                label = label.strong();
            }
            let clicked = ui.add_enabled_ui(row.enabled, |ui| ui.selectable_label(selected, label).clicked()).inner;
            if clicked {
                keys.push(LanKey::Click(at));
            }
            ui.horizontal(|ui| value(ui, row, at, keys));
            ui.end_row();
        }
    });
}

/// The value column of a row: chevrons around a choice, the typed text in the selection colour, or the value.
fn value(ui: &mut egui::Ui, row: &Row, at: usize, keys: &mut Vec<LanKey>) {
    match row.kind {
        RowKind::Choice => {
            if ui.add_enabled(row.enabled, egui::Button::new("<").small()).clicked() {
                keys.push(LanKey::Step(at, false));
            }
            ui.label(&row.value);
            if ui.add_enabled(row.enabled, egui::Button::new(">").small()).clicked() {
                keys.push(LanKey::Step(at, true));
            }
        }
        RowKind::Text { editing: true } => {
            ui.label(RichText::new(&row.value).monospace().color(ui.visuals().selection.stroke.color));
        }
        _ => {
            ui.label(RichText::new(&row.value).weak());
        }
    }
}

fn side_column(ui: &mut egui::Ui, page: &PageView) {
    ui.set_width(SIDE_WIDTH);
    ui.label(RichText::new(&page.side_title).strong());
    if page.side.is_empty() {
        ui.label(RichText::new("No messages yet").weak());
    }
    for line in &page.side {
        ui.label(RichText::new(line).small());
    }
}

/// Draws the session HUD: the feed bottom left, the vote and the countdown at the top, the spectator panel top right.
pub fn show_hud(ctx: &egui::Context, hud: &HudView) {
    let text = ctx.global_style().visuals.text_color();
    let weak = ctx.global_style().visuals.weak_text_color();
    if !hud.feed.is_empty() {
        egui::Area::new(egui::Id::new("lan_feed"))
            .anchor(Align2::LEFT_BOTTOM, vec2(16.0, -140.0))
            .interactable(false)
            .show(ctx, |ui| {
                for (kind, line, alpha) in &hud.feed {
                    let colour = match kind {
                        FeedKind::Chat => text,
                        FeedKind::Notice => weak,
                    };
                    let shadowed = RichText::new(line)
                        .color(colour.gamma_multiply(*alpha))
                        .background_color(Color32::from_black_alpha((120.0 * alpha) as u8));
                    ui.label(shadowed);
                }
            });
    }
    let banners = hud.countdown.iter().map(|line| (line, 32.0)).chain(hud.vote.iter().map(|line| (line, 16.0)));
    egui::Area::new(egui::Id::new("lan_banners")).anchor(Align2::CENTER_TOP, vec2(0.0, 72.0)).interactable(false).show(
        ctx,
        |ui| {
            for (line, size) in banners {
                ui.label(RichText::new(line).size(size).strong().background_color(Color32::from_black_alpha(140)));
            }
        },
    );
    let Some(spectator) = &hud.spectator else { return };
    egui::Window::new("lan_spectator")
        .title_bar(false)
        .resizable(false)
        .interactable(false)
        .anchor(Align2::RIGHT_TOP, vec2(-16.0, 120.0))
        .frame(Frame::window(&ctx.global_style()))
        .show(ctx, |ui| {
            ui.label(RichText::new("Spectating").strong());
            ui.label(&spectator.following);
            ui.separator();
            for line in &spectator.board {
                ui.label(RichText::new(line).monospace());
            }
        });
}

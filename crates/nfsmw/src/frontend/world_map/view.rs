//! The map's egui view, drawn over the paused world by the devtools pass: the picture (zoomed and panned by a UV crop),
//! the blips of the selected type with the HUD's tints, the attached blip's card, the list of that type on the right,
//! and the "Activate GPS?" prompt.

use egui::{
    Align2, Color32, CornerRadius, FontId, Id, LayerId, Order, Pos2, Rect, RichText, Stroke, TextureHandle,
    TextureOptions, pos2, vec2,
};

use super::control::{format_metres, icon_name, style_name, tint_of};
use super::distance_metres;
use super::layout::picture_rect;
use super::overlay::{Picture, Pictures, WorldMapOverlay};

/// The uploaded textures: the city picture and one per blip icon file.
pub struct Textures {
    pub map: Option<TextureHandle>,
    pub icons: Vec<(&'static str, TextureHandle)>,
}

/// The size of a blip's icon in points.
const ICON: f32 = 22.0;
/// The width of the list on the right, in points.
const PANEL_WIDTH: f32 = 280.0;

/// Draws the open map. Does nothing while it is closed.
pub fn show(ctx: &egui::Context, overlay: &mut WorldMapOverlay) {
    if !overlay.open {
        return;
    }
    upload(ctx, overlay);
    let screen = ctx.content_rect();
    let [x, y, side, _] = picture_rect(screen.width(), screen.height());
    let picture = Rect::from_min_size(pos2(x, y), vec2(side, side));
    let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("world_map")));
    painter.rect_filled(screen, CornerRadius::ZERO, Color32::from_black_alpha(235));
    draw_picture(&painter, overlay, picture);
    draw_blips(&painter, overlay, picture);
    draw_header(&painter, overlay, picture, screen);
    draw_card(&painter, overlay, picture);
    draw_list(ctx, overlay, picture);
    draw_prompt(ctx, overlay);
}

/// Uploads the pictures the first time the map opens (`overlay.pending`).
fn upload(ctx: &egui::Context, overlay: &mut WorldMapOverlay) {
    if overlay.textures.is_some() {
        return;
    }
    let pending = overlay.pending.take().unwrap_or_default();
    let Pictures { map, icons } = pending;
    overlay.textures = Some(Textures {
        map: map.map(|p| load(ctx, "world_map_picture", &p)),
        icons: icons.into_iter().map(|(name, p)| (name, load(ctx, name, &p))).collect(),
    });
}

fn load(ctx: &egui::Context, name: &str, picture: &Picture) -> TextureHandle {
    let image = egui::ColorImage::from_rgba_unmultiplied(picture.size, &picture.rgba);
    ctx.load_texture(name, image, TextureOptions::LINEAR)
}

/// The window point of a view position on the picture.
fn at(rect: Rect, view: [f32; 2]) -> Pos2 {
    pos2(rect.min.x + view[0] * rect.width(), rect.min.y + view[1] * rect.height())
}

/// The part of the city picture the view shows: the square of half-width `0.5 / zoom` around the centre.
fn draw_picture(painter: &egui::Painter, overlay: &WorldMapOverlay, rect: Rect) {
    let camera = overlay.session.camera;
    let Some(texture) = overlay.textures.as_ref().and_then(|t| t.map.as_ref()) else {
        painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_gray(40));
        return;
    };
    let half = 0.5 / camera.zoom;
    let uv = Rect::from_min_max(
        pos2(camera.centre[0] - half, camera.centre[1] - half),
        pos2(camera.centre[0] + half, camera.centre[1] + half),
    );
    painter.image(texture.id(), rect, uv, Color32::WHITE);
}

/// The blips of the selected type that are inside the view. The attached one gets a ring.
fn draw_blips(painter: &egui::Painter, overlay: &WorldMapOverlay, rect: Rect) {
    let camera = overlay.session.camera;
    for (index, blip) in overlay.blips.iter().enumerate() {
        if !overlay.session.filter.contains(blip.style) {
            continue;
        }
        let view = camera.to_view(blip.map);
        if view.iter().any(|v| !(0.0..=1.0).contains(v)) {
            continue;
        }
        let centre = at(rect, view);
        let tint = tint_of(blip.style).map_or(Color32::WHITE, |[r, g, b]| Color32::from_rgb(r, g, b));
        match icon(overlay, blip.style) {
            Some(texture) => {
                let icon_rect = Rect::from_center_size(centre, vec2(ICON, ICON));
                painter.image(texture, icon_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), tint);
            }
            None => {
                painter.circle_filled(centre, 6.0, tint);
            }
        }
        if overlay.session.attached == Some(index) {
            painter.circle_stroke(centre, ICON * 0.75, Stroke::new(2.0, Color32::WHITE));
        }
    }
}

fn icon(overlay: &WorldMapOverlay, style: &str) -> Option<egui::TextureId> {
    let name = icon_name(style);
    let textures = overlay.textures.as_ref()?;
    textures.icons.iter().find(|(n, _)| *n == name).map(|(_, t)| t.id())
}

fn draw_header(painter: &egui::Painter, overlay: &WorldMapOverlay, rect: Rect, screen: Rect) {
    let camera = overlay.session.camera;
    painter.text(
        pos2(rect.min.x, rect.min.y - 10.0),
        Align2::LEFT_BOTTOM,
        "WORLD MAP",
        FontId::proportional(20.0),
        Color32::WHITE,
    );
    painter.text(
        pos2(rect.max.x, rect.min.y - 10.0),
        Align2::RIGHT_BOTTOM,
        format!("Zoom x{:.1}", camera.zoom),
        FontId::proportional(14.0),
        Color32::from_gray(200),
    );
    painter.text(
        pos2(screen.center().x, screen.max.y - 12.0),
        Align2::CENTER_BOTTOM,
        "Scroll: zoom   Z: zoom all   Drag: pan   A / D: tab   Enter or right click: GPS   Esc or M: close",
        FontId::proportional(14.0),
        Color32::from_gray(200),
    );
}

/// The attached blip's name, its icon and its distance, under the picture.
fn draw_card(painter: &egui::Painter, overlay: &WorldMapOverlay, rect: Rect) {
    let Some(index) = overlay.session.attached else { return };
    let Some(blip) = overlay.blips.get(index) else { return };
    let distance = format_metres(distance_metres(overlay.player, blip));
    let at = pos2(rect.min.x, rect.max.y + 14.0);
    let text = format!("{}   {}", style_name(blip.style), distance);
    painter.text(at, Align2::LEFT_TOP, text, FontId::proportional(18.0), Color32::WHITE);
}

/// The list of the selected type: every blip of it, its name and distance, the attached one highlighted.
fn draw_list(ctx: &egui::Context, overlay: &WorldMapOverlay, rect: Rect) {
    let filter = overlay.session.filter;
    let origin = pos2(rect.max.x + 36.0, rect.min.y);
    egui::Area::new(Id::new("world_map_list")).fixed_pos(origin).show(ctx, |ui| {
        ui.set_width(PANEL_WIDTH);
        ui.label(RichText::new(filter.label().to_uppercase()).size(20.0).strong());
        ui.label(RichText::new("A / D or the arrows change the tab").weak());
        ui.add_space(8.0);
        let listed = overlay.session.listed(&overlay.blips);
        if listed.is_empty() {
            ui.label("No blips of this type on the map.");
        }
        for index in listed {
            let blip = &overlay.blips[index];
            let distance = format_metres(distance_metres(overlay.player, blip));
            let row = format!("{}   {}", style_name(blip.style), distance);
            let text = match overlay.session.attached == Some(index) {
                true => RichText::new(row).size(16.0).color(Color32::YELLOW),
                false => RichText::new(row).size(16.0),
            };
            ui.label(text);
        }
    });
}

/// The "Activate GPS?" prompt over the attached blip. Enter confirms; Esc or M cancels.
fn draw_prompt(ctx: &egui::Context, overlay: &WorldMapOverlay) {
    if !overlay.session.prompt {
        return;
    }
    let Some(blip) = overlay.session.attached.and_then(|i| overlay.blips.get(i)) else { return };
    egui::Window::new("Activate GPS?")
        .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("Set the GPS route to the {}?", style_name(blip.style).to_lowercase()));
            ui.label(RichText::new("Enter: yes   Esc: no").weak());
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_point_is_inside_the_picture_rectangle() {
        let rect = Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 200.0));
        assert_eq!(at(rect, [0.0, 0.0]), pos2(10.0, 20.0));
        assert_eq!(at(rect, [1.0, 1.0]), pos2(110.0, 220.0));
    }
}

//! The cluster's own places for the values (`[[bike.dash.field]]`): each rectangle is erased with the picture's own background and the
//! speed, the gear or the revs drawn in it in the cluster's colour, scaled to the rectangle, so the live screen is the baked cluster showing
//! live values and not a panel of the game's own.

use nfsmw_data::car::mods::bike_dash::{Align, Channel, Field};

use super::{Canvas, DashState, SEGMENTS, text_width};

/// The text a field shows for `state`: the speed in whole km/h, the gear as `N`, `R` or its number.
fn text_of(channel: Channel, state: &DashState) -> String {
    match channel {
        Channel::Speed => (state.speed_kmh.abs().round().min(999.0) as u32).to_string(),
        Channel::Gear => match state.gear {
            -1 => "R".to_owned(),
            0 => "N".to_owned(),
            n => n.to_string(),
        },
        Channel::RpmBar => String::new(),
    }
}

/// The RGBA picture of `field` for `state`, the size of its rectangle.
pub(super) fn paint_field(field: &Field, state: &DashState) -> Vec<u8> {
    let [_, _, width, height] = field.rect;
    let mut canvas = Canvas { width, height, rgba: vec![255; (width * height * 4) as usize] };
    canvas.fill((0, 0, width, height), field.background);
    match field.channel {
        Channel::RpmBar => bar(&mut canvas, field, state),
        channel => text(&mut canvas, field, &text_of(channel, state)),
    }
    canvas.rgba
}

/// `text` as large as the rectangle allows (the glyphs are 5 by 7 dots) and justified as the field says.
fn text(canvas: &mut Canvas, field: &Field, text: &str) {
    let (width, height) = (canvas.width, canvas.height);
    let glyphs = text.chars().count().max(1) as u32;
    // As high as the cluster's own font (the field's `height`), else as high as the rectangle allows; never wider than the rectangle.
    let high = match field.height {
        0 => height * 9 / 10,
        own => own.min(height),
    };
    let unit = (high / 7).min(width / (6 * glyphs - 1).max(1)).max(1);
    let used = text_width(unit, text);
    let x = match field.align {
        Align::Left => 0,
        Align::Center => width.saturating_sub(used) / 2,
        Align::Right => width.saturating_sub(used),
    };
    canvas.text((x, height.saturating_sub(7 * unit) / 2), unit, field.color, text);
}

/// The revs: the share of the bar from the left in the lit colour, in as many segments as the cluster's bar has (1: a continuous bar).
fn bar(canvas: &mut Canvas, field: &Field, state: &DashState) {
    let (width, height) = (canvas.width, canvas.height);
    let share = (state.rpm / state.rpm_max.max(1.0)).clamp(0.0, 1.0);
    let segments = field.segments.clamp(1, SEGMENTS);
    if segments == 1 {
        canvas.fill((0, 0, (width as f32 * share).round() as u32, height), field.color);
        return;
    }
    let lit = (share * segments as f32).ceil() as u32;
    let pitch = width / segments;
    for segment in 0..lit.min(segments) {
        canvas.fill((segment * pitch, 0, (pitch * 3 / 4).max(1), height), field.color);
    }
}

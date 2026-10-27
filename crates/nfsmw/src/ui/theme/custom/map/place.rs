//! Where the map sits: at the top of the content box, horizontally centred, its top edge on the safe area's top (3.5 % of
//! the height from the screen's edge), and what that asks of the elements beside and under it.
//!
//! - **The ellipse.** A mode's half-axes are fractions of the content box's height (`modes.rs`). The horizontal one is
//!   held back where the top columns need the room: the radio card and the race and pursuit panels are up to
//!   [`SIDE_RESERVE`] units wide at scale 1 and sit at the box's top corners, so the map never reaches them, down to a
//!   floor of [`MIN_HALF_WIDTH`] on a very narrow window. The vertical one follows, keeping the ellipse's shape.
//! - **The centre.** The ellipse's top is fixed on the safe area's top, so a bigger mode grows downwards.
//! - **The clearance.** Everything else at the top centre (the race strip, the lines under it, the turn chip, the
//!   countdown, the end card) stacks under the map: [`top_clearance`] is how far down the map takes the top centre, and
//!   `HudBox::with_top_clearance` applies it.

use glam::Vec2;

use super::modes::Overlay;
use crate::settings::hud::HudSettings;
use crate::ui::theme::custom::layout::HudBox;

/// The widest of the top-left and top-right columns at scale 1, canvas units (the radio card).
pub const SIDE_RESERVE: f32 = 240.0;
/// The room kept between the map and a top column, and between the map and what stacks under it.
pub const SIDE_GAP: f32 = 6.0;
pub const TOP_GAP: f32 = 6.0;
/// The narrowest half-width the map is squeezed to, canvas units.
pub const MIN_HALF_WIDTH: f32 = 50.0;

/// The ellipse's half-axes in canvas units for `fractions` (of the content box's height) in `frame`: the fractions
/// scaled to the box, then held back so the map clears the top columns. The shape is kept.
pub fn radii(fractions: [f32; 2], frame: &HudBox) -> Vec2 {
    let wanted = Vec2::new(fractions[0], fractions[1]) * frame.content().h;
    if wanted.x <= 0.0 {
        return wanted;
    }
    let inset = frame.safe().min_x() - frame.content().min_x();
    let room = frame.content().w / 2.0 - inset - SIDE_RESERVE * frame.scale() - SIDE_GAP;
    let reach = room.max(MIN_HALF_WIDTH);
    wanted * (reach / wanted.x).min(1.0)
}

/// The ellipse's centre: the content box's middle across, and `radii.y` below the safe area's top.
pub fn centre(frame: &HudBox, radii: Vec2) -> Vec2 {
    Vec2::new(frame.content().x, frame.safe().min_y() + radii.y)
}

/// How far down the map takes the top centre in `settings`' settled mode, canvas units: the map's height and the gap under
/// it, or nothing with the map off.
pub fn top_clearance(settings: &HudSettings, frame: &HudBox) -> f32 {
    let overlay = Overlay::of(settings.map_mode, settings.map_opacity);
    if overlay.shown <= 0.0 {
        return 0.0;
    }
    2.0 * radii(overlay.radii, frame).y + TOP_GAP
}

/// `frame` with the top centre cleared for the map `settings` choose.
pub fn cleared(frame: HudBox, settings: &HudSettings) -> HudBox {
    let clearance = top_clearance(settings, &frame);
    frame.with_top_clearance(clearance)
}

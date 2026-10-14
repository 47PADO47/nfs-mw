//! An abstract map blip (`docs/specs/hud-minimap.md` section 7). A [`BlipStyle`] names its icons, decides when a blip
//! of that style shows, and gives its look on every tick, so a custom blip is one `impl` and an animation is just
//! the look changing with the tick. The binding in [`super::blips`] places them; this file has no runtime.

/// One thing on the map, in world space (metres, render space: x forward, y left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapBlip {
    /// The style's [`BlipStyle::name`]. A blip whose style is not registered is not drawn.
    pub style: &'static str,
    pub position: [f32; 2],
    /// Any length; the icon points along it.
    pub heading: [f32; 2],
    /// The condition the style reads as its state: the cop style flashes while this is set.
    pub active: bool,
}

/// When a blip of a style is drawn at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Always eligible: drawn at the disc's rim when it is outside the circle.
    Always,
    /// Drawn only while the minimap view covers the blip's own place (the view has entered its zone). Outside the
    /// circle it is not drawn at all, rather than pinned to the rim.
    InZone,
}

/// How a blip looks on one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlipLook {
    /// The tint as `[red, green, blue, alpha]`. `None` keeps the package object's own colour.
    pub colour: Option<[u8; 4]>,
    /// Multiplies the placement's fade, 0..1.
    pub alpha: f32,
    /// Added to the icon's rotation, degrees: a spinning icon changes this every tick.
    pub spin: f32,
}

impl BlipLook {
    /// The package object's own colour, opaque, not turned.
    pub const PLAIN: BlipLook = BlipLook { colour: None, alpha: 1.0, spin: 0.0 };
}

/// What a kind of blip is. Implement it for a custom blip; register it with the HUD binding.
pub trait BlipStyle: Send + Sync {
    /// Unique; a [`MapBlip`] names its style with it.
    fn name(&self) -> &'static str;
    /// The HUD object names of this style's pool, in order: one object per blip shown at once.
    fn icons(&self) -> Vec<String>;
    /// The condition: whether this blip shows at all. Default: always.
    fn shows(&self, _blip: &MapBlip) -> bool {
        true
    }
    /// Whether the blips of this style are always eligible, or only while the view covers their zone. Default: always.
    fn visibility(&self) -> Visibility {
        Visibility::Always
    }
    /// Whether only the nearest blip of this style to the player is drawn. Default: all of them.
    fn nearest_only(&self) -> bool {
        false
    }
    /// The tint used where the package object is plain white (so the game's own tint, if any, is kept). Default: none.
    fn fallback_colour(&self) -> Option<[u8; 4]> {
        None
    }
    /// The look at `tick` (the binding's frame counter). Default: the package's colour, not turned.
    fn look(&self, _blip: &MapBlip, _tick: u32) -> BlipLook {
        BlipLook::PLAIN
    }
}

/// The colour to set on a pool object whose package colour is `package`, for a look and the style's fallback.
/// A look's own colour always wins. A plain white object takes the fallback. Anything else keeps its colour.
pub(super) fn colour_to_apply(look: &BlipLook, fallback: Option<[u8; 4]>, package: [u8; 4]) -> Option<[u8; 4]> {
    if let Some(colour) = look.colour {
        return Some(colour);
    }
    let plain_white = package[..3] == [0xff, 0xff, 0xff];
    if !plain_white {
        return None;
    }
    fallback
}

/// Whether a blip with its own place inside the circle (`inside`) is drawn in `visibility`.
pub(super) fn drawn_in(visibility: Visibility, inside: bool) -> bool {
    match visibility {
        Visibility::Always => true,
        Visibility::InZone => inside,
    }
}

/// The look of `blip` in `style` at `tick`, or `None` when its condition hides it.
pub(super) fn look_of(style: &dyn BlipStyle, blip: &MapBlip, tick: u32) -> Option<BlipLook> {
    if !style.shows(blip) {
        return None;
    }
    Some(style.look(blip, tick))
}

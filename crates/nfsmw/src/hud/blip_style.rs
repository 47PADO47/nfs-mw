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

/// How a blip looks on one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlipLook {
    pub colour: [u8; 4],
    /// Multiplies the placement's fade, 0..1.
    pub alpha: f32,
    /// Added to the icon's rotation, degrees: a spinning icon changes this every tick.
    pub spin: f32,
}

impl BlipLook {
    /// An opaque white icon, not turned.
    pub const PLAIN: BlipLook = BlipLook { colour: [0xff, 0xff, 0xff, 0xff], alpha: 1.0, spin: 0.0 };
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
    /// The look at `tick` (the binding's frame counter). Default: a plain white icon.
    fn look(&self, _blip: &MapBlip, _tick: u32) -> BlipLook {
        BlipLook::PLAIN
    }
}

/// The look of `blip` in `style` at `tick`, or `None` when its condition hides it.
pub(super) fn look_of(style: &dyn BlipStyle, blip: &MapBlip, tick: u32) -> Option<BlipLook> {
    if !style.shows(blip) {
        return None;
    }
    Some(style.look(blip, tick))
}

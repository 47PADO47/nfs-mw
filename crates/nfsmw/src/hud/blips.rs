//! The map blips on the minimap: every [`BlipStyle`] registered here gets a pool of HUD objects from the package, and
//! each frame every blip of a registered style is placed with `blackbox_minimap::place_blip`. A blip whose condition
//! hides it, that is not drawn in its style's visibility, or that lands outside the disc, takes no object; a drawn
//! blip past the pool is not drawn. The look comes from the style, on the binding's tick (`docs/specs/hud-minimap.md`
//! section 7).

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::blip_style::{BlipStyle, MapBlip, TICK_SECONDS, colour_to_apply, drawn_in, look_of};
use super::blip_styles;
use super::minimap::MinimapBinding;
use super::state::HudState;

/// One HUD object of a style's pool, with the colour the package gave it.
struct Slot {
    style: usize,
    object: ObjectRef,
    package: [u8; 4],
}

/// The registered styles, their pools and the frame counter that animates them.
pub struct BlipBinding {
    styles: Vec<Box<dyn BlipStyle>>,
    slots: Vec<Slot>,
    /// Seconds since the binding began, advanced by the frame time. The look tick is this over `TICK_SECONDS`.
    elapsed: f32,
}

impl BlipBinding {
    /// The game's own styles.
    pub fn builtin(rt: &mut Runtime, package: PackageId) -> Self {
        Self::new(rt, package, blip_styles::builtin())
    }

    /// Registers `styles`, finds their pools' objects and hides them all.
    pub fn new(rt: &mut Runtime, package: PackageId, styles: Vec<Box<dyn BlipStyle>>) -> Self {
        let mut binding = Self { styles: Vec::new(), slots: Vec::new(), elapsed: 0.0 };
        for style in styles {
            binding.add(rt, package, style);
        }
        binding
    }

    /// The name hashes of the pool objects of the styles registered now (see `HudViewport::new`).
    pub fn icon_hashes(&self) -> Vec<u32> {
        self.styles.iter().flat_map(|style| style.icons()).map(|name| fe_hash_upper(&name)).collect()
    }

    /// Registers one more style. A style whose name is already registered is ignored.
    pub fn add(&mut self, rt: &mut Runtime, package: PackageId, style: Box<dyn BlipStyle>) {
        if self.styles.iter().any(|s| s.name() == style.name()) {
            return;
        }
        let index = self.styles.len();
        let icons = style.icons();
        let mut found = 0;
        for name in &icons {
            let Some(object) = rt.find(package, fe_hash_upper(name)) else {
                continue;
            };
            rt.set_hidden(object, true);
            let package = rt.object(object).map_or([0xff; 4], |state| state.data.colour_rgba());
            self.slots.push(Slot { style: index, object, package });
            found += 1;
        }
        // A style whose icons are not in the package stays registered: it draws once its objects exist.
        log::info!("blip style {}: {found} of {} pool objects in the HUD package", style.name(), icons.len());
        self.styles.push(style);
    }

    /// Moves the looks on by `dt` seconds. Call once per frame.
    pub fn advance(&mut self, dt: f32) {
        self.elapsed += dt;
    }

    /// Places the blips of `state` on the minimap. Call before `Runtime::update`, after the minimap itself.
    pub fn apply(&mut self, rt: &mut Runtime, state: &HudState, minimap: &MinimapBinding) {
        let tick = (self.elapsed / TICK_SECONDS) as u32;
        self.hide_all(rt);
        let Some(at) = state.minimap else {
            return;
        };
        let view = minimap.view(state, &at);
        let centre = minimap.centre();
        let mut used = vec![0usize; self.styles.len()];
        // The nearest blip of each nearest-only style, by its index in `state.blips`.
        let nearest: Vec<Option<usize>> = self
            .styles
            .iter()
            .map(
                |style| if style.nearest_only() { nearest_blip(&state.blips, style.name(), at.position) } else { None },
            )
            .collect();
        for (bi, blip) in state.blips.iter().enumerate() {
            let Some(index) = self.styles.iter().position(|s| s.name() == blip.style) else {
                continue;
            };
            let style = self.styles[index].as_ref();
            if style.nearest_only() && nearest[index] != Some(bi) {
                continue;
            }
            let Some(look) = look_of(style, blip, tick) else {
                continue;
            };
            // Every blip turns with the player's arrow, whatever way it is heading.
            let placed = minimap.blip(&view, at.orientation, blip.position, at.heading);
            if !drawn_in(style.visibility(blip), placed.inside) {
                continue;
            }
            let Some(slot) = self.slots.iter().filter(|s| s.style == index).nth(used[index]) else {
                continue;
            };
            let o = slot.object;
            let alpha = placed.alpha * look.alpha;
            // Only a blip that is drawn takes an object: one out of the disc must not hide a nearer one.
            if alpha <= 0.0 {
                continue;
            }
            used[index] += 1;
            rt.set_hidden(o, false);
            rt.set_position_xy(o, centre[0] + placed.offset[0], centre[1] + placed.offset[1]);
            rt.set_rotation_z(o, (placed.rotation + look.spin).to_radians());
            // The colour's alpha would overwrite the fade, so the colour goes first and the fade last.
            if let Some(colour) = colour_to_apply(&look, style.fallback_colour(), slot.package) {
                rt.set_colour_rgba(o, colour);
            }
            rt.set_alpha(o, (alpha * 255.0) as u8);
        }
    }

    fn hide_all(&self, rt: &mut Runtime) {
        for slot in &self.slots {
            rt.set_hidden(slot.object, true);
        }
    }
}

/// The index in `blips` of the blip of `style` nearest to `from` (render space), or `None` when there is none.
pub(super) fn nearest_blip(blips: &[MapBlip], style: &str, from: [f32; 2]) -> Option<usize> {
    let distance = |b: &MapBlip| {
        let (dx, dy) = (b.position[0] - from[0], b.position[1] - from[1]);
        dx * dx + dy * dy
    };
    blips
        .iter()
        .enumerate()
        .filter(|(_, b)| b.style == style)
        .min_by(|(_, a), (_, b)| distance(a).total_cmp(&distance(b)))
        .map(|(i, _)| i)
}

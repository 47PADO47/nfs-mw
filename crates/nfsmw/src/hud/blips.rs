//! The map blips on the minimap: every [`BlipStyle`] registered here gets a pool of HUD objects from the package, and
//! each frame every blip of a registered style is placed with `blackbox_minimap::place_blip`. A blip whose condition
//! hides it takes no object; a place beyond the pool is not drawn. The look comes from the style, on the binding's
//! tick (`docs/specs/hud-minimap.md` section 7).

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::blip_style::{BlipStyle, look_of};
use super::blip_styles;
use super::minimap::MinimapBinding;
use super::state::HudState;

/// One HUD object of a style's pool.
struct Slot {
    style: usize,
    object: ObjectRef,
}

/// The registered styles, their pools and the frame counter that animates them.
pub struct BlipBinding {
    styles: Vec<Box<dyn BlipStyle>>,
    slots: Vec<Slot>,
    tick: u32,
}

impl BlipBinding {
    /// The game's own styles.
    pub fn builtin(rt: &mut Runtime, package: PackageId) -> Self {
        Self::new(rt, package, blip_styles::builtin())
    }

    /// Registers `styles`, finds their pools' objects and hides them all.
    pub fn new(rt: &mut Runtime, package: PackageId, styles: Vec<Box<dyn BlipStyle>>) -> Self {
        let mut binding = Self { styles: Vec::new(), slots: Vec::new(), tick: 0 };
        for style in styles {
            binding.add(rt, package, style);
        }
        binding
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
            self.slots.push(Slot { style: index, object });
            found += 1;
        }
        // A style whose icons are not in the package stays registered: it draws once its objects exist.
        log::info!("blip style {}: {found} of {} pool objects in the HUD package", style.name(), icons.len());
        self.styles.push(style);
    }

    /// Places the blips of `state` on the minimap. Call before `Runtime::update`, after the minimap itself.
    pub fn apply(&mut self, rt: &mut Runtime, state: &HudState, minimap: &MinimapBinding) {
        let tick = self.tick;
        self.tick = self.tick.wrapping_add(1);
        self.hide_all(rt);
        let Some(at) = state.minimap else {
            return;
        };
        let view = minimap.view(state, &at);
        let centre = minimap.centre();
        let mut used = vec![0usize; self.styles.len()];
        for blip in &state.blips {
            let Some(index) = self.styles.iter().position(|s| s.name() == blip.style) else {
                continue;
            };
            let Some(look) = look_of(self.styles[index].as_ref(), blip, tick) else {
                continue;
            };
            let Some(slot) = self.slots.iter().filter(|s| s.style == index).nth(used[index]) else {
                continue;
            };
            used[index] += 1;
            let placed = minimap.blip(&view, at.orientation, blip.position, blip.heading);
            let alpha = placed.alpha * look.alpha;
            if alpha <= 0.0 {
                continue;
            }
            let o = slot.object;
            rt.set_hidden(o, false);
            rt.set_position_xy(o, centre[0] + placed.offset[0], centre[1] + placed.offset[1]);
            rt.set_rotation_z(o, (placed.rotation + look.spin).to_radians());
            rt.set_alpha(o, (alpha * 255.0) as u8);
            rt.set_colour_rgba(o, look.colour);
        }
    }

    fn hide_all(&self, rt: &mut Runtime) {
        for slot in &self.slots {
            rt.set_hidden(slot.object, true);
        }
    }
}

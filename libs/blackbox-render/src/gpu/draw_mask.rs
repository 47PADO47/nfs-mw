//! Which draw lists a draw belongs in, worked out once per draw when its mesh is created. The passes then
//! test one bitmask per draw instead of comparing its blend and shading against every pipeline.

use super::draw_list::{BUCKETS, combinations, takes};
use super::pipelines::BLEND_ORDER;
use crate::{DrawRange, Shading};

// The colour buckets take the low bits, then one bit per blend mode for the normal-map and window lists,
// then one for the sky list.
const NORMAL_SHIFT: usize = BUCKETS;
const WINDOW_SHIFT: usize = NORMAL_SHIFT + BLEND_ORDER.len();
const SKY_BIT: usize = WINDOW_SHIFT + BLEND_ORDER.len();
const _: () = assert!(SKY_BIT < u64::BITS as usize);

/// The draw lists one draw goes into.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct DrawMask(u64);

impl DrawMask {
    /// The lists `draw` goes into: every colour bucket whose blend it matches and whose shading takes it, the
    /// normal-map or window list of its blend mode, and the sky list.
    pub(super) fn of(draw: &DrawRange) -> Self {
        let mut bits = 0u64;
        for (slot, (blend, shading)) in combinations().enumerate() {
            if draw.blend == blend && takes(draw.shading, shading) {
                bits |= 1 << slot;
            }
        }
        let Some(blend) = BLEND_ORDER.iter().position(|&b| b == draw.blend) else { return Self(bits) };
        match draw.shading {
            Shading::WorldNormal(_) => bits |= 1 << (NORMAL_SHIFT + blend),
            Shading::WorldWindow(_) => bits |= 1 << (WINDOW_SHIFT + blend),
            Shading::SkyLayer(_) => bits |= 1 << SKY_BIT,
            _ => {}
        }
        Self(bits)
    }

    /// The colour buckets (slots in [`combinations`] order) the draw goes into.
    pub(super) fn colour_slots(self) -> impl Iterator<Item = usize> {
        let mut bits = self.0 & ((1 << BUCKETS) - 1);
        std::iter::from_fn(move || {
            if bits == 0 {
                return None;
            }
            let slot = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            Some(slot)
        })
    }

    /// Whether the draw goes into the normal-map list of blend mode number `blend` (in `BLEND_ORDER`).
    pub(super) fn normal(self, blend: usize) -> bool {
        self.0 & (1 << (NORMAL_SHIFT + blend)) != 0
    }

    /// Whether the draw goes into the window list of blend mode number `blend` (in `BLEND_ORDER`).
    pub(super) fn window(self, blend: usize) -> bool {
        self.0 & (1 << (WINDOW_SHIFT + blend)) != 0
    }

    /// Whether the draw is a sky layer.
    pub(super) fn sky(self) -> bool {
        self.0 & (1 << SKY_BIT) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlendMode, GlossyMaterialHandle};
    use blackbox_gfx::{NormalMaterialHandle, SkyMaterialHandle, WindowMaterialHandle};

    /// The blend mode's number in `BLEND_ORDER`, which is its list index.
    fn blend_index(blend: BlendMode) -> usize {
        BLEND_ORDER.iter().position(|&b| b == blend).expect("known blend mode")
    }

    fn draw(blend: BlendMode, shading: Shading) -> DrawRange {
        DrawRange { first_index: 0, index_count: 3, base_vertex: 0, texture: None, blend, shading, two_sided: true }
    }

    #[test]
    fn colour_slots_match_the_old_per_bucket_test() {
        let shadings = [
            Shading::Lit,
            Shading::Prelit,
            Shading::World,
            Shading::Sky,
            Shading::Glossy(GlossyMaterialHandle::from_raw(1)),
            Shading::SkyLayer(SkyMaterialHandle::from_raw(1)),
            Shading::WorldNormal(NormalMaterialHandle::from_raw(1)),
            Shading::WorldWindow(WindowMaterialHandle::from_raw(1)),
        ];
        for blend in BLEND_ORDER {
            for shading in shadings {
                let d = draw(blend, shading);
                let expected: Vec<usize> = combinations()
                    .enumerate()
                    .filter(|&(_, (b, s))| d.blend == b && takes(d.shading, s))
                    .map(|(slot, _)| slot)
                    .collect();
                assert_eq!(DrawMask::of(&d).colour_slots().collect::<Vec<_>>(), expected, "{blend:?} {shading:?}");
            }
        }
    }

    #[test]
    fn normal_window_and_sky_bits_follow_shading_and_blend() {
        for blend in BLEND_ORDER {
            let at = blend_index(blend);
            let normal = DrawMask::of(&draw(blend, Shading::WorldNormal(NormalMaterialHandle::from_raw(2))));
            let window = DrawMask::of(&draw(blend, Shading::WorldWindow(WindowMaterialHandle::from_raw(2))));
            let sky = DrawMask::of(&draw(blend, Shading::SkyLayer(SkyMaterialHandle::from_raw(2))));
            let lit = DrawMask::of(&draw(blend, Shading::Lit));
            for other in 0..BLEND_ORDER.len() {
                assert_eq!(normal.normal(other), other == at);
                assert_eq!(window.window(other), other == at);
                assert!(!window.normal(other) && !normal.window(other) && !lit.normal(other) && !lit.window(other));
            }
            assert!(sky.sky() && !normal.sky() && !window.sky() && !lit.sky());
        }
    }

    #[test]
    fn a_plain_draw_is_in_exactly_its_own_bucket() {
        let mask = DrawMask::of(&draw(BlendMode::Opaque, Shading::Lit));
        assert_eq!(mask.colour_slots().count(), 1);
        assert_eq!(DrawMask::default().colour_slots().count(), 0);
    }
}

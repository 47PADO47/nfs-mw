//! Which faces of a texture's surfaces are drawn, from the texture pack's info record. See
//! `docs/specs/backface-culling.md`.

use blackbox_tpk::{AlphaUsage, Texture};

/// `TextureInfo::Flags` bit that keeps both faces of the surfaces drawn with the texture.
const TWO_SIDED_FLAG: u8 = 0x02;

/// What a texture says about the faces of its surfaces. Single-sided surfaces cull their back faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sidedness {
    /// A copy-mode (unblended) texture keeps both faces: punch-through alpha, or the info record's two-sided flag.
    pub two_sided: bool,
    /// Blended (`AlphaBlendType` other than copy): both faces, unless the effect culls blended surfaces. Then
    /// `two_sided` does not matter.
    pub blended: bool,
}

impl Sidedness {
    /// The sidedness of `t`. A missing texture keeps both faces.
    pub fn of(t: Option<&Texture>) -> Self {
        let Some(t) = t else { return Self { two_sided: true, blended: false } };
        Self {
            two_sided: t.alpha_usage == AlphaUsage::PunchThrough || t.flags & TWO_SIDED_FLAG != 0,
            blended: t.alpha_blend != 0,
        }
    }

    /// Whether a surface keeps both faces, drawn by an effect that culls blended surfaces (`culls_blended`) or not.
    pub fn draws_both(self, culls_blended: bool) -> bool {
        if self.blended {
            return !culls_blended;
        }
        self.two_sided
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_tpk::PixelFormat;

    fn texture(alpha_usage: AlphaUsage, alpha_blend: u8, flags: u8) -> Texture {
        Texture {
            name: String::new(),
            name_hash: 0,
            width: 4,
            height: 4,
            mip_levels: 1,
            format: PixelFormat::Argb8888,
            compression_type: 0,
            alpha_usage,
            alpha_sorting: false,
            alpha_blend,
            flags,
            data: Vec::new(),
            palette: Vec::new(),
        }
    }

    #[test]
    fn opaque_textures_are_single_sided_unless_flagged() {
        assert!(!Sidedness::of(Some(&texture(AlphaUsage::None, 0, 0))).draws_both(false));
        assert!(!Sidedness::of(Some(&texture(AlphaUsage::Modulated, 0, 1))).draws_both(false), "bit 0 is not it");
        assert!(Sidedness::of(Some(&texture(AlphaUsage::None, 0, 2))).draws_both(true));
        assert!(Sidedness::of(Some(&texture(AlphaUsage::PunchThrough, 0, 0))).draws_both(true));
    }

    #[test]
    fn blended_textures_are_two_sided_unless_the_effect_culls_them() {
        let glass = Sidedness::of(Some(&texture(AlphaUsage::Modulated, 1, 0)));
        assert!(glass.draws_both(false));
        assert!(!glass.draws_both(true));
        let flagged = Sidedness::of(Some(&texture(AlphaUsage::PunchThrough, 2, 2)));
        assert!(!flagged.draws_both(true), "the flag and the cut-out do not count for blended surfaces");
        assert!(Sidedness::of(None).draws_both(true), "an unknown texture draws both faces");
    }
}

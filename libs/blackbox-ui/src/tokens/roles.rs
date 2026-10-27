//! The colour roles a theme may declare beyond the five core colours: the linework of a plan or map, the guide path over
//! it, the meaning colours and the two layers that dim and hold content over a busy scene.
//!
//! A theme declares a role as `Some(colour)`, or leaves it `None` and gets a colour derived from its core tokens
//! ([`Roles::resolve`]). So a theme written before the roles existed, and a mod theme whose ABI carries only the core
//! colours, still gives every role a value that matches the theme's own palette.

/// The roles a theme may set. A role left `None` is derived from the core colours when the tokens are written to the
/// skin.
///
/// - `line`, `line_minor`, `line_faint`: the main, the lesser and the faintest linework of a plan or map (roads, side
///   roads, paths).
/// - `guide`: the path that guides the user over the linework (a route).
/// - `success`, `caution`, `danger`, `info`: the meaning colours. A state also has a shape or a word; colour is never the
///   only carrier.
/// - `scrim`: dims everything behind a modal layer.
/// - `overlay`: the strong plate that holds content on top of the scrim.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Roles {
    pub line: Option<[u8; 4]>,
    pub line_minor: Option<[u8; 4]>,
    pub line_faint: Option<[u8; 4]>,
    pub guide: Option<[u8; 4]>,
    pub success: Option<[u8; 4]>,
    pub caution: Option<[u8; 4]>,
    pub danger: Option<[u8; 4]>,
    pub info: Option<[u8; 4]>,
    pub scrim: Option<[u8; 4]>,
    pub overlay: Option<[u8; 4]>,
}

/// Every role with a colour: what the skin carries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedRoles {
    pub line: [u8; 4],
    pub line_minor: [u8; 4],
    pub line_faint: [u8; 4],
    pub guide: [u8; 4],
    pub success: [u8; 4],
    pub caution: [u8; 4],
    pub danger: [u8; 4],
    pub info: [u8; 4],
    pub scrim: [u8; 4],
    pub overlay: [u8; 4],
}

/// The meaning colours a theme gets when it names none: a green, an orange, a red and a blue.
pub const DEFAULT_SUCCESS: [u8; 4] = [125, 206, 160, 255];
pub const DEFAULT_CAUTION: [u8; 4] = [232, 135, 58, 255];
pub const DEFAULT_DANGER: [u8; 4] = [240, 68, 82, 255];
pub const DEFAULT_INFO: [u8; 4] = [90, 150, 255, 255];
/// The alpha of a derived scrim, and the least alpha of a derived overlay.
pub const SCRIM_ALPHA: u8 = 165;
pub const OVERLAY_ALPHA: u8 = 224;
/// How much of the background's brightness a derived scrim keeps.
pub const SCRIM_KEEP: f32 = 0.4;

/// The five core colours the roles derive from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Core {
    pub primary: [u8; 4],
    pub secondary: [u8; 4],
    pub background: [u8; 4],
    pub border: [u8; 4],
    pub muted: [u8; 4],
}

impl Roles {
    /// Every role: the declared colour, else the one derived from `core`.
    ///
    /// - `line` is the primary colour, `line_minor` the muted one, `line_faint` the border, `guide` the secondary.
    /// - The meaning colours are the neutral defaults ([`DEFAULT_SUCCESS`] and the others).
    /// - `scrim` is the background darkened to [`SCRIM_KEEP`] of its brightness, at [`SCRIM_ALPHA`].
    /// - `overlay` is the background at least [`OVERLAY_ALPHA`] opaque.
    pub fn resolve(&self, core: Core) -> ResolvedRoles {
        ResolvedRoles {
            line: self.line.unwrap_or(core.primary),
            line_minor: self.line_minor.unwrap_or(core.muted),
            line_faint: self.line_faint.unwrap_or(core.border),
            guide: self.guide.unwrap_or(core.secondary),
            success: self.success.unwrap_or(DEFAULT_SUCCESS),
            caution: self.caution.unwrap_or(DEFAULT_CAUTION),
            danger: self.danger.unwrap_or(DEFAULT_DANGER),
            info: self.info.unwrap_or(DEFAULT_INFO),
            scrim: self.scrim.unwrap_or_else(|| derived_scrim(core.background)),
            overlay: self.overlay.unwrap_or_else(|| derived_overlay(core.background)),
        }
    }
}

/// The scrim a background derives: its colour darkened, at [`SCRIM_ALPHA`].
pub fn derived_scrim([r, g, b, _]: [u8; 4]) -> [u8; 4] {
    let keep = |c: u8| (f32::from(c) * SCRIM_KEEP).round() as u8;
    [keep(r), keep(g), keep(b), SCRIM_ALPHA]
}

/// The overlay a background derives: its colour, at least [`OVERLAY_ALPHA`] opaque.
pub fn derived_overlay([r, g, b, a]: [u8; 4]) -> [u8; 4] {
    [r, g, b, a.max(OVERLAY_ALPHA)]
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORE: Core = Core {
        primary: [250, 250, 250, 255],
        secondary: [10, 200, 100, 255],
        background: [20, 40, 60, 100],
        border: [1, 2, 3, 4],
        muted: [100, 110, 120, 255],
    };

    #[test]
    fn an_undeclared_role_derives_from_the_core_colours() {
        let roles = Roles::default().resolve(CORE);
        assert_eq!(roles.line, CORE.primary);
        assert_eq!(roles.line_minor, CORE.muted);
        assert_eq!(roles.line_faint, CORE.border);
        assert_eq!(roles.guide, CORE.secondary);
        assert_eq!(roles.scrim, [8, 16, 24, SCRIM_ALPHA]);
        assert_eq!(roles.overlay, [20, 40, 60, OVERLAY_ALPHA]);
        assert_eq!(roles.danger, DEFAULT_DANGER);
    }

    #[test]
    fn a_declared_role_wins_and_the_others_still_derive() {
        let roles = Roles { guide: Some([1, 1, 1, 255]), ..Roles::default() }.resolve(CORE);
        assert_eq!(roles.guide, [1, 1, 1, 255]);
        assert_eq!(roles.line, CORE.primary);
    }

    #[test]
    fn an_overlay_never_drops_below_the_least_alpha_but_keeps_a_stronger_one() {
        assert_eq!(derived_overlay([0, 0, 0, 0])[3], OVERLAY_ALPHA);
        assert_eq!(derived_overlay([0, 0, 0, 250])[3], 250);
    }

    #[test]
    fn the_scrim_is_always_stronger_than_a_light_hud_plate() {
        assert!(
            derived_scrim([0, 0, 0, 0])[3] > 128 && derived_overlay([0, 0, 0, 0])[3] > derived_scrim([0, 0, 0, 0])[3]
        );
    }
}

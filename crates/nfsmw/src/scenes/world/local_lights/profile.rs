//! The enhanced lamp profile in one table: inspected layout constants and artistic light values.
//! None of these are recovered original physical-lamp parameters.
use std::ops::Range;

/// Shared lantern texture whose original UV layout places the glass panes in one band.
pub(in crate::scenes::world) const LANTERN_TEXTURE: u32 = 3295825187;

/// The glass region of [`LANTERN_TEXTURE`] on a 64-unit normalized grid, so replacement artwork of any resolution
/// masks the same panes.
pub(super) struct GlassMask {
    pub grid: u64,
    pub x: Range<u64>,
    pub y: Range<u64>,
}
pub(super) const GLASS_MASK: GlassMask = GlassMask { grid: 64, x: 4..60, y: 5..44 };

/// A lantern mesh variant whose glass triangles were inspected in the installed data.
pub(in crate::scenes::world) struct GlassLayout {
    /// Upper-case solid names this layout applies to.
    pub models: &'static [&'static str],
    /// Index count of the textured lantern-head group that holds the glass.
    pub group_indices: u32,
    /// Index count of the glass triangles within it.
    pub glass_indices: usize,
    /// A glass triangle's V coordinates lie within `band` and extend below `reach.start` and above `reach.end`.
    pub band: Range<f32>,
    pub reach: Range<f32>,
}

/// The `_1Z_` iron lanterns and the `_1C_`/`_1Z_` park lantern have no triangle that covers only the glass band (their
/// panes share triangles with the roof), so they are not split and keep a glow sprite.
pub(super) const GLASS_LAYOUTS: &[GlassLayout] = &[
    GlassLayout {
        models: &[
            "XO_IRONLAMPSTANDING_1B_00",
            "XO_IRONLAMPSTANDINGB_1B_00",
            "XO_IRONLAMPHANGING_1B_00",
            "XO_PARKLAMPA_1B_00",
        ],
        group_indices: 90,
        glass_indices: 30,
        band: 0.1288..0.6681,
        reach: 0.13..0.66,
    },
    GlassLayout {
        models: &["XO_IRONLAMPSTANDING_1C_00", "XO_IRONLAMPSTANDINGB_1C_00", "XO_IRONLAMPHANGING_1C_00"],
        group_indices: 54,
        glass_indices: 18,
        band: 0.1288..0.6681,
        reach: 0.13..0.66,
    },
];

/// The artistic spotlight of a recognized fixture.
pub(super) struct Emitter {
    pub color: [f32; 3],
    pub intensity: f32,
    pub outer_cos: f32,
    pub inner_cos: f32,
    /// Reach of tunnel fixtures; street fixtures use the persisted street-light reach.
    pub fixture_range: f32,
}
pub(super) const EMITTER: Emitter =
    Emitter { color: [1.0, 0.94, 0.80], intensity: 0.85, outer_cos: 0.20, inner_cos: 0.65, fixture_range: 18.0 };

/// A street fixture tilted further than this from upright (cosine to world up) is treated as knocked over.
pub(super) const UPRIGHT_COS: f32 = 0.65;
/// Visible street bulbs have their own budget and distance, beyond the surface-light and shadow budgets.
pub(super) const GLOW_BUDGET: usize = 256;
pub(super) const GLOW_DISTANCE: f32 = 400.0;
/// Lights and bulbs fade in over this many metres inside their selection boundary.
pub(super) const FADE_METRES: f32 = 32.0;

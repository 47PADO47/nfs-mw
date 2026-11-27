//! Which world textures texture variation may break up (`RenderBackend::set_texture_variation`).
//!
//! Large tiled ground and wall textures repeat every 10–60 m, which reads as a grid on hillsides and long walls. Only
//! named textures that tile over most of their placed area qualify: open grass, golf-course grass, hill fields, two
//! cliff faces and grunge ground. Plain concretes are left out: their textures also clad building facades, where
//! the shifted panel joints show. Prefixes are not enough: the same families hold facades, windows, edge strips
//! (`_LE_`), alpha-tested hill silhouettes and one-off concrete props, which a shifted read visibly breaks.

/// Exact (stored, 23-character) texture names with the variation cell in texture repeats.
const TILED: [(&str, f32); 9] = [
    ("TRNS_GRASSA_LD_GRASSA_0", 1.5),
    ("TRNS_GRASSC_LD_GRASSC_0", 1.5),
    ("TRNS_GCGRASS_LD_ROUGH_0", 1.5),
    ("TRNS_GCGRASS_LD_FAREWAY", 1.5),
    ("TRNS_GCGRASS_LX_PUTTING", 1.5),
    ("TRN_CT_HILLS_FIELDS", 1.0),
    ("TRNS_CLIFFC_LU_02", 1.0),
    ("TRNS_ALL_LF_CLIFFD_02", 1.0),
    ("TRNS_GRUNGA_LD_GRUNGA_0", 1.5),
];

/// The variation cell of the world texture called `name`, or `None` to keep it as authored.
pub(crate) fn cell(name: &str) -> Option<f32> {
    TILED.iter().find(|(known, _)| name.eq_ignore_ascii_case(known)).map(|(_, cell)| *cell)
}

#[cfg(test)]
mod tests {
    use super::cell;

    #[test]
    fn terrain_cliffs_and_concrete_vary_but_strips_and_buildings_do_not() {
        assert_eq!(cell("TRNS_GRASSA_LD_GRASSA_0"), Some(1.5));
        assert_eq!(cell("TRNS_ALL_LF_CLIFFD_02"), Some(1.0));
        assert_eq!(cell("ARC_CONCRET_01"), None, "also a building facade");
        assert_eq!(cell("ARC_CONCRETEPOLE_VC"), None, "a prop, not a tiled surface");
        assert_eq!(cell("ARC_CAMPUS_MB_TOWER4"), None, "a facade");
        assert_eq!(cell("TRN_CT_HILLS2"), None, "an alpha-tested silhouette");
        assert_eq!(cell("TRNS_GRASSA_LE_GRASSA_C"), None, "an edge strip");
        assert_eq!(cell("TRNS_GRASSA_LC_PAVEMENT"), None, "a pavement strip");
        assert_eq!(cell("TRNS_ALL_LZ_SIDEWALKA_0"), None);
        assert_eq!(cell("ARC_BRICKBLD_BROWN_LZ"), None, "a building atlas");
        assert_eq!(cell("ORG_CT_POPLARFLORA"), None);
    }
}

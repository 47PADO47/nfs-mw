//! Content categories independent of colour visibility and renderer material families.
pub(super) fn casts_sun(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    !["PAN_", "SKY", "SHD_", "SHADOW_"].iter().any(|prefix| upper.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn background_art_is_not_a_physical_occluder() {
        for name in
            ["PAN_ISLAND_1A_08", "PAN_Island_1a_08", "PAN_CT_TERRAINA_02", "SKYDOME", "SHD_ROAD", "SHADOW_TUNNEL"]
        {
            assert!(!casts_sun(name), "{name}");
        }
        for name in ["TRN_CT_ROAD", "TRN_CT_TERRAIN", "XT_CT_FALLTREES", "BLD_WAREHOUSE", "PROP_BIN"] {
            assert!(casts_sun(name), "{name}");
        }
    }
}

//! Verified lamp-head mesh groups for the enhanced local-light system.

/// Returns the index count of the fixture group, or zero for an unrecognized model.
pub(super) fn lens_indices(name: &str) -> u32 {
    match name {
        "XO_STREETLIGHTC_1B_00"
        | "XO_STREETLIGHTC_1C_00"
        | "XO_STREETLIGHTCB_1B_00"
        | "XO_STREETLIGHTCN_1B_00"
        | "XO_STREETLIGHTCC_1B_00"
        | "XO_STREETLIGHTD_1B_00"
        | "XO_STREETLIGHTD2_1B_00"
        | "XO_STREETLIGHT_ITALYJG_1B_00"
        | "XO_STREETLIGHT_ITALYJG_1C_00"
        | "XO_STREETLIGHT_ITALYJG_2B_00"
        | "XO_STREETLIGHT_ITALYJG_2C_00"
        | "XO_TRAFFICLIGHTA_1B_00"
        | "XO_TRAFFICLIGHTA_1C_00"
        | "XO_TRAFFICLIGHTB_1B_00"
        | "XO_TRAFFICLIGHTB_1C_00" => 36,
        "XO_TRAFFICLIGHTA_1Z_00" | "XO_TRAFFICLIGHTB_1Z_00" => 18,
        "XO_STREETLIGHTDRIFT_1B_00" | "XO_STREETLIGHTDRIFT_1C_00" => 30,
        "XO_LIGHTPOSTA_1A_00" => 12,
        "XO_LIGHTPOSTB_1A_00" | "XO_LIGHTPOSTB_1C_00" | "XO_LIGHTPOSTB_1Z_00" | "XO_LIGHTPOSTB_WALL_1B_00" => 24,
        "XO_PARKLAMPA_1B_00"
        | "XO_IRONLAMPHANGING_1B_00"
        | "XO_IRONLAMPSTANDING_1B_00"
        | "XO_IRONLAMPSTANDINGB_1B_00" => 90,
        "XO_IRONLAMPHANGING_1C_00" | "XO_IRONLAMPSTANDING_1C_00" | "XO_IRONLAMPSTANDINGB_1C_00" => 54,
        "XO_PARKLAMPA_1C_00"
        | "XO_IRONLAMPHANGING_1Z_00"
        | "XO_IRONLAMPSTANDING_1Z_00"
        | "XO_IRONLAMPSTANDINGB_1Z_00" => 36,
        "XO_PARKLAMPA_1Z_00" => 18,
        "XO_STREETLIGHTI_1B_00" => 96,
        "XO_STREETLIGHTI_1C_00" | "XO_STREETLIGHTI_1Z_00" => 48,
        _ => 0,
    }
}

/// The diffuse texture of the catalogued lens group, inspected with its index count. `XO_STREETLIGHTI_` has a second
/// group of the same size, so the count alone does not identify the lens.
pub(super) fn lens_texture(name: &str) -> Option<u32> {
    let family = name.split("_1").next()?;
    match family {
        "XO_STREETLIGHTC"
        | "XO_STREETLIGHTCB"
        | "XO_STREETLIGHTCN"
        | "XO_STREETLIGHTCC"
        | "XO_TRAFFICLIGHTA"
        | "XO_TRAFFICLIGHTB"
        | "XO_LIGHTPOSTB"
        | "XO_LIGHTPOSTB_WALL"
        | "XO_STREETLIGHT_ITALYJG"
        | "XO_STREETLIGHT_ITALYJG_2B_00"
        | "XO_STREETLIGHT_ITALYJG_2C_00" => Some(535917023),
        "XO_STREETLIGHTD" | "XO_STREETLIGHTD2" => Some(4237971217),
        "XO_STREETLIGHTI" | "XO_LIGHTPOSTA" => Some(3755337936),
        "XO_STREETLIGHTDRIFT" => Some(1297190669),
        "XO_PARKLAMPA" | "XO_IRONLAMPHANGING" | "XO_IRONLAMPSTANDING" | "XO_IRONLAMPSTANDINGB" => {
            Some(super::profile::LANTERN_TEXTURE)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_catalogued_model_has_a_lens_texture() {
        for family in ["XO_STREETLIGHTC_1B_00", "XO_STREETLIGHT_ITALYJG_2C_00", "XO_LIGHTPOSTB_WALL_1B_00"] {
            assert_eq!(super::lens_texture(family), Some(535917023), "{family}");
        }
        assert_eq!(super::lens_texture("XO_STREETLIGHTI_1Z_00"), Some(3755337936));
        assert_eq!(super::lens_texture("XO_IRONLAMPSTANDINGB_1C_00"), Some(super::super::profile::LANTERN_TEXTURE));
        assert_eq!(super::lens_texture("XO_STREETLIGHTZ_1B_00"), None);
    }
}

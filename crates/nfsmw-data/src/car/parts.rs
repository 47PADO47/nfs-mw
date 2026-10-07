//! Which of a car's solids make up the stock car. Car solids are named
//! `<CAR>_<PART>_<LOD>`, e.g. `BMWM3GTR_KIT00_FRONT_BRAKE_A` (docs/formats/models.md).

/// Whether a solid belongs to the stock car at `lod`.
pub fn is_stock_part(name: &str, lod: char, all_parts: bool) -> bool {
    let suffix = format!("_{lod}");
    if !name.ends_with(&suffix) {
        return false;
    }
    if all_parts {
        return true;
    }
    // Decals need vinyl textures we don't load yet; KIT00 is the stock body kit.
    !name.contains("_DECAL_") && (!name.contains("_KIT") || name.contains("_KIT00_"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_part_filter() {
        assert!(is_stock_part("BMWM3GTR_BASE_A", 'A', false));
        assert!(is_stock_part("BMWM3GTR_KIT00_BODY_A", 'A', false));
        assert!(!is_stock_part("BMWM3GTR_KIT01_BODY_A", 'A', false));
        assert!(!is_stock_part("BMWM3GTR_KIT00_BODY_B", 'A', false));
        assert!(!is_stock_part("BMWM3GTR_KIT00_DECAL_LEFT_DOOR_RECT_MEDIUM_A", 'A', false));
        assert!(is_stock_part("BMWM3GTR_KIT00_DECAL_LEFT_DOOR_RECT_MEDIUM_A", 'A', true));
    }
}

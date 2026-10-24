//! Native decal filter identity; color is defined by part attributes rather than bitmap names.

use blackbox_hash::bstring_hash_continue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecalVariant {
    pub brand: u32,
    pub name: u32,
}

impl DecalVariant {
    pub fn matches(self, black: bool) -> bool {
        let expected = match black {
            true => self.brand,
            false => bstring_hash_continue(self.brand, b"_WHITE"),
        };
        self.name == expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_hash::bstring_hash;

    #[test]
    fn native_decal_color_uses_brand_name_identity_and_hash_continuation() {
        let brand = bstring_hash("EXAMPLE_MANUFACTURER");
        let black = DecalVariant { brand, name: brand };
        let white = DecalVariant { brand, name: bstring_hash("EXAMPLE_MANUFACTURER_WHITE") };
        assert!(black.matches(true));
        assert!(!black.matches(false));
        assert!(white.matches(false));
        assert!(!white.matches(true));
        let custom = DecalVariant { brand, name: bstring_hash("CUSTOM_WHITE_ARTWORK") };
        assert!(!custom.matches(true) && !custom.matches(false));
    }

    #[test]
    #[ignore = "requires installed decal part attributes"]
    fn installed_decal_filter_metadata_matches_native_brand_and_name_attributes() {
        use crate::car::{CarTables, customization::Inventory};
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let tables = CarTables::load(&dir).unwrap();
        let inventory = Inventory::load(&dir).unwrap();
        let catalog = inventory.catalog(&dir, "COBALTSS").unwrap();
        let mut colors = [0; 2];
        let mut paired = 0;
        for decal in &catalog.decals {
            for choice in &decal.choices {
                let part = tables.parts.by_name_hash(choice.hash).unwrap();
                let variant = decal.variants[&choice.hash];
                assert_eq!(Some(variant.brand), tables.parts.attribute(part, "BRAND_NAME"));
                assert_eq!(Some(variant.name), tables.parts.attribute(part, "NAME"));
                for black in [true, false] {
                    if !variant.matches(black) {
                        continue;
                    }
                    colors[usize::from(!black)] += 1;
                    if decal.variants.values().any(|other| other.brand == variant.brand && other.matches(!black)) {
                        paired += 1;
                    }
                }
            }
        }
        assert!(colors.into_iter().all(|count| count > 50), "black/white counts {colors:?}");
        assert!(paired > 100, "only {paired} paired variants");
    }
}

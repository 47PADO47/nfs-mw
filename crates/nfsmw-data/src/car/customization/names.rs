//! Installed cart names that are independent of geometry and price validation.

use super::{CarTables, Catalog, Category, Choice};

pub(super) fn read(tables: &CarTables, catalog: &mut Catalog) {
    let hashes: Vec<_> = catalog
        .stock
        .into_iter()
        .flatten()
        .chain(catalog.choices.iter().flatten().map(|choice| choice.hash))
        .chain(catalog.decals.iter().flat_map(|zone| zone.choices.iter().map(|choice| choice.hash)))
        .chain(catalog.number_choices.iter().flatten().map(|choice| choice.hash))
        .collect();
    for hash in hashes {
        let Some(part) = tables.parts.by_name_hash(hash) else { continue };
        catalog
            .native_names
            .insert(hash, (tables.parts.name(part).to_owned(), tables.parts.attribute(part, "LANGUAGEHASH")));
    }
    for category in [Category::Paint, Category::RimPaint] {
        for choice in &catalog.choices[category as usize] {
            let Some(part) = tables.parts.by_name_hash(choice.hash) else { continue };
            let finish = match tables.parts.attribute(part, "BRAND_NAME").unwrap_or_default() {
                0x02da_ab07 => 0xb676_3cde,
                0x0379_7533 => 0xb715_070a,
                0x0000_da27 => 0xb310_0a3e,
                _ => 0x452b_5481,
            };
            catalog
                .paint_names
                .insert(choice.hash, (finish, tables.parts.attribute(part, "SPEECHCOLOUR").unwrap_or_default()));
        }
    }
    for part in tables.parts.parts().iter().filter(|part| part.part_id == 82) {
        let Some(index) = tables.parts.attribute(part, "HUDINDEX").and_then(|index| u8::try_from(index).ok()) else {
            continue;
        };
        catalog.gauge_names.entry(index).or_insert_with(|| Choice {
            hash: part.name_hash,
            label: tables.parts.name(part).to_owned(),
            language_hash: tables.parts.attribute(part, "LANGUAGEHASH"),
            carbon: false,
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires an installed game"]
    fn installed_garage_cart_paint_and_gauge_names_keep_native_language_metadata() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let catalog = super::Catalog::load(&dir, "COBALTSS").unwrap();
        assert!(catalog.paint_names.values().any(|(finish, color)| *finish != 0 && *color != 0));
        assert!(!catalog.gauge_names.is_empty());
        for stock in catalog.stock.into_iter().flatten() {
            assert!(catalog.native_names.contains_key(&stock), "stock part names must be retained for native captions");
        }
        for skin in catalog.gauge_skins.iter().copied().filter(|skin| *skin > 0) {
            let name = catalog.gauge_names.get(&skin).expect("installed HUD must have a native part name");
            assert!(name.language_hash.is_some() || !name.label.is_empty());
        }
    }
}

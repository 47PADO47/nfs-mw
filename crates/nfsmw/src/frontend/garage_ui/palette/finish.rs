use super::{Palette, Part};

pub(super) fn family_page(family: &str) -> &str {
    match family {
        "GLOSS" | "METAL" | "PEARL" => family,
        _ => "CUSTOM",
    }
}

pub(super) fn page_for(catalog: &nfsmw_data::car::customization::Catalog, hash: u32) -> Option<&str> {
    if let Some(&(_, brand)) = catalog.paint_swatch_order.get(&hash) {
        return match brand {
            0x02da_ab07 => Some("GLOSS"),
            0x0343_7a52 => Some("METAL"),
            0x0379_7533 => Some("CUSTOM"),
            _ => None,
        };
    }
    catalog.paint_families.get(&hash).map(|name| family_page(name))
}

impl Palette {
    pub(super) fn sort_swatches(&mut self, catalog: &nfsmw_data::car::customization::Catalog, category: Part) {
        self.colors.sort_by_key(|swatch| {
            swatch
                .hash
                .or(catalog.stock[category as usize])
                .and_then(|hash| catalog.paint_swatch_order.get(&hash))
                .map_or(usize::MAX, |entry| entry.0)
        });
    }

    pub(super) fn family_label(&self) -> String {
        if self.color() == 1 {
            return crate::ui::Text::Game(0xb310_0a3e).resolve().unwrap_or_else(|| "Powder Coat".into());
        }
        let family = &self.families[self.family];
        if family == "CUSTOM" {
            return crate::i18n::text("options-custom", &crate::i18n::Args::new());
        }
        let label = (self.color() == 0 && matches!(family.as_str(), "GLOSS" | "METAL" | "PEARL"))
            .then(|| {
                let model = self.handle.model();
                let catalog = model.selected()?.parts.as_ref()?;
                let choice = catalog.choices[Part::Paint as usize]
                    .iter()
                    .find(|choice| catalog.paint_families.get(&choice.hash) == Some(family))?;
                let (label, _) = catalog.paint_names.get(&choice.hash)?;
                crate::ui::Text::Game(*label).resolve()
            })
            .flatten();
        label.unwrap_or_else(|| family.clone())
    }
}

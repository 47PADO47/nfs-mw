use super::{Palette, Part};

pub(super) fn family_page(family: &str) -> &str {
    match family {
        "GLOSS" | "METAL" | "PEARL" => family,
        _ => "CUSTOM",
    }
}

impl Palette {
    pub(super) fn family_label(&self) -> String {
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

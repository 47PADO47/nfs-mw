//! The order and the names the developer's `parts` command, the garage panel and `view-car --part` list a car's choices in,
//! so that a number means the same part in each. A mod car's choices (its own paints, rims, liveries) are listed the same
//! way as a stock car's.

use nfsmw_data::car::customization::{Catalog, Category};

/// Give each choice its name (the language's, when the choice has a label of the game; body kits and rims keep theirs), put
/// the choices of each category in name order, and tell choices that share a name apart.
pub(crate) fn arrange(catalog: &mut Catalog, language: &dyn Fn(u32) -> Option<String>) {
    for (index, choices) in catalog.choices.iter_mut().enumerate() {
        for choice in choices.iter_mut() {
            if index != Category::Body as usize
                && index != Category::Wheels as usize
                && let Some(name) = choice.language_hash.and_then(language)
                && !name.trim().is_empty()
            {
                choice.label = name;
            }
            if choice.carbon {
                choice.label.push_str(" (Carbon)");
            }
        }
        choices.sort_by(|a, b| (&a.label, a.hash).cmp(&(&b.label, b.hash)));
        // Some installed variants share a localized name. Keep every identity selectable.
        let labels: Vec<_> = choices.iter().map(|p| p.label.clone()).collect();
        for (i, choice) in choices.iter_mut().enumerate() {
            let duplicate = labels.iter().filter(|label| **label == labels[i]).count() > 1;
            if duplicate {
                let variant = labels[..=i].iter().filter(|label| **label == labels[i]).count();
                choice.label.push_str(&format!(" · {variant}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nfsmw_data::car::customization::Choice;

    fn choice(hash: u32, label: &str) -> Choice {
        Choice { hash, label: label.into(), language_hash: None, carbon: false }
    }

    #[test]
    fn a_mod_cars_liveries_are_listed_in_name_order() {
        let mut catalog = Catalog::default();
        let liveries = &mut catalog.choices[Category::Vinyl as usize];
        liveries.extend([choice(3, "Livery 3"), choice(1, "Livery 1"), choice(2, "Livery 2")]);
        arrange(&mut catalog, &|_| None);
        let labels: Vec<_> = catalog.choices[Category::Vinyl as usize].iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["Livery 1", "Livery 2", "Livery 3"]);
    }

    #[test]
    fn choices_that_share_a_name_are_numbered() {
        let mut catalog = Catalog::default();
        catalog.choices[Category::Paint as usize].extend([choice(1, "Gloss"), choice(2, "Gloss")]);
        arrange(&mut catalog, &|_| None);
        let labels: Vec<_> = catalog.choices[Category::Paint as usize].iter().map(|c| c.label.clone()).collect();
        assert_eq!(labels, ["Gloss · 1", "Gloss · 2"]);
    }

    #[test]
    fn a_game_label_names_a_choice_but_not_a_body_kit() {
        let mut catalog = Catalog::default();
        let named = Choice { language_hash: Some(7), ..choice(1, "RAW_PART") };
        catalog.choices[Category::Spoiler as usize].push(named.clone());
        catalog.choices[Category::Body as usize].push(named);
        arrange(&mut catalog, &|hash| (hash == 7).then(|| "Big wing".to_owned()));
        assert_eq!(catalog.choices[Category::Spoiler as usize][0].label, "Big wing");
        assert_eq!(catalog.choices[Category::Body as usize][0].label, "RAW_PART");
    }
}

//! Display names come from the language's game labels (`crate::i18n::game`); model folder names remain the fallback.

use blackbox_text::label_hash;

pub(crate) fn display(
    folder: &str,
    manufacturer: &str,
    brands: &[String],
    label_text: &dyn Fn(u32) -> Option<String>,
) -> (String, String) {
    if let Some(name) = police_name(folder) {
        return (name.to_owned(), "Police".to_owned());
    }
    let upper = folder.to_ascii_uppercase();
    for brand in std::iter::once(manufacturer.to_owned()).chain(brands.iter().cloned()) {
        let brand = key_brand(&brand);
        let short = upper.strip_prefix(&brand).unwrap_or(&upper);
        for model in [upper.as_str(), short] {
            let label = format!("CARNAME_{brand}_{model}");
            if let Some(name) = label_text(label_hash(&label)) {
                return (name, pretty_brand(&brand));
            }
        }
    }
    let compact = upper.replace('_', "");
    let name = match compact.as_str() {
        "BMWM3GTR" => "M3 GTR",
        "BMWM3GTRE46" => "M3 GTR (E46)",
        "CORVETTE" => "Corvette C6",
        "CORVETTEC6R" => "Corvette C6.R",
        "MONARO" => "Monaro",
        "RX8SPEEDT" => "RX-8 variant",
        "SLR" => "SLR McLaren",
        "CARRERAGT" => "Carrera GT",
        "IMPREZAWRX" => "Impreza WRX",
        _ => folder,
    };
    (name.replace('_', " "), pretty_brand(&key_brand(manufacturer)))
}

fn key_brand(brand: &str) -> String {
    let brand = brand.to_ascii_uppercase();
    if brand == "CHEVEROLET" {
        return "CHEVROLET".to_owned();
    }
    brand.replace([' ', '_'], "")
}

fn pretty_brand(brand: &str) -> String {
    match brand {
        "BMW" => "BMW".to_owned(),
        "ASTONMARTIN" => "Aston Martin".to_owned(),
        "VOLKSWAGEN" => "Volkswagen".to_owned(),
        _ => {
            let mut letters = brand.chars();
            let first = letters.next().map_or(String::new(), |letter| letter.to_uppercase().to_string());
            first + &letters.as_str().to_ascii_lowercase()
        }
    }
}

fn police_name(folder: &str) -> Option<&'static str> {
    Some(match folder {
        "COPMIDSIZE" => "Patrol cruiser",
        "COPGHOST" => "Undercover cruiser",
        "COPGTO" => "GTO pursuit",
        "COPGTOGHOST" => "Undercover GTO",
        "COPSPORT" => "Corvette pursuit",
        "COPSPORTGHOST" => "Undercover Corvette",
        "COPSPORTHENCH" => "Corvette pursuit variant",
        "COPSUV" => "Police SUV",
        "COPSUVL" => "Police SUV variant",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch16_display_names_normalize_physics_alias_case_and_separators() {
        assert_eq!(display("carrera_gt", "Porsche", &[], &|_| None), ("Carrera GT".into(), "Porsche".into()));
        assert_eq!(display("bmwm3gtre46", "BMW", &[], &|_| None), ("M3 GTR (E46)".into(), "BMW".into()));
    }
}

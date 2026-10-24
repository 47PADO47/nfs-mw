//! Display names come from the language's game labels (`crate::i18n::game`); model folder names remain the fallback.

use blackbox_text::label_hash;

pub(super) fn display(
    folder: &str,
    manufacturer: &str,
    brands: &[String],
    label_text: &dyn Fn(u32) -> Option<String>,
) -> (String, String) {
    if let Some(name) = police_name(folder) {
        return (name.to_owned(), "Police".to_owned());
    }
    for brand in std::iter::once(manufacturer.to_owned()).chain(brands.iter().cloned()) {
        let brand = key_brand(&brand);
        let short = folder.strip_prefix(&brand).unwrap_or(folder);
        for model in [folder, short] {
            let label = format!("CARNAME_{brand}_{model}");
            if let Some(name) = label_text(label_hash(&label)) {
                return (name, pretty_brand(&brand));
            }
        }
    }
    let name = match folder {
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

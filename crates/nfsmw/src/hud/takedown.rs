//! The takedown line: the generic message the HUD shows when the player wrecks a cop car in a chase, with the car's name
//! and the bounty the wreck added (`Civic Cruiser` over `Combo Bounty: 2,500`). Which label each cop car takes, what the
//! number is and how the line plays are in `docs/specs/hud-pursuit-takedown.md`.

use super::race::Message;
use crate::i18n::{self, fmt::grouped};

/// The language label of each cop car's takedown line, by vehicle name **[exe]**. Both of Cross's cars take his line; a
/// car not listed has no line.
const LABELS: [(&str, u32); 11] = [
    ("copcross", 0x8FE0_2B9F),
    ("copsport", 0x8FE0_2B9F),
    ("copmidsize", 0xF49A_550A),
    ("copghost", 0x9023_11DA),
    ("copgto", 0x9BFD_379F),
    ("copgtoghost", 0x65E9_7524),
    ("copsporthench", 0x4EE0_7213),
    ("copsportghost", 0x4ED0_0512),
    ("copsuv", 0x9BFD_6AD3),
    ("copsuvpatrol", 0x9BFD_6AD3),
    ("copsuvl", 0x1BAA_C57F),
];

/// The label of `car`'s takedown line (the vehicle name, in any case); `None` for a car with no line.
pub fn label_of(car: &str) -> Option<u32> {
    LABELS.iter().find(|(name, _)| name.eq_ignore_ascii_case(car)).map(|&(_, label)| label)
}

/// The line from the label's text: its `^` breaks the line, and its `%$d` is the bounty with the thousands separator.
pub fn fill(label_text: &str, bounty: u32) -> String {
    label_text.replace('^', "\n").replace("%$d", &grouped(i64::from(bounty)))
}

/// The takedown line of the chase's `count`th wreck, a `car` that added `bounty`; `None` for a car with no line or a
/// label the language does not have.
pub fn message(count: u32, car: &str, bounty: u32) -> Option<Message> {
    let text = i18n::game(label_of(car)?)?;
    Some(Message::takedown(count, fill(&text, bounty)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_cop_car_takes_its_own_line_and_crosss_cars_share_his() {
        assert_eq!(label_of("copmidsize"), Some(0xF49A_550A));
        assert_eq!(label_of("COPGHOST"), Some(0x9023_11DA), "the name is matched in any case");
        assert_eq!(label_of("copcross"), label_of("copsport"));
        assert_eq!(label_of("copsuv"), label_of("copsuvpatrol"));
        assert_eq!(label_of("copheli"), None, "the helicopter has no line");
        assert_eq!(label_of("traffictaxi"), None);
    }

    #[test]
    fn the_label_breaks_at_its_caret_and_takes_the_grouped_bounty() {
        assert_eq!(fill("Civic Cruiser^Combo Bounty: %$d", 2500), "Civic Cruiser\nCombo Bounty: 2,500");
        assert_eq!(fill("SUV Heavy^Combo Bounty: %$d", 750), "SUV Heavy\nCombo Bounty: 750");
    }
}

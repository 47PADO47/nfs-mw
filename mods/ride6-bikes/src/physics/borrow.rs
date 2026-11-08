//! Data the dump does not hold, taken from a similar model, as named choices (`docs/modding/ride6-bikes-batch.md`, "Refused bikes").
//!
//! Two bikes of the install have a field RIDE 6 itself leaves empty: the rain tyres of the Indian Challenger RR (`TireSections` is
//! three unset references in the game's own asset) and the Italjet Dragster 200 CC (`RakeTrailOverride` is an explicitly empty list).
//! The importer does not guess a number: it takes the field from the nearest model of the same data and says so, in the log and in a
//! comment of the manifest. The choosers here are plain functions on names and numbers so the rules are tested with authored data
//! (`borrow_tests.rs`); the files they read are [`super::vehicle`]'s.

/// One field taken from another asset: what it is, where it came from and by which rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Borrowed {
    /// The field, named like the manifest's reader would look for it (`FrontTyre_Rain.TireSections`, `Skeleton.RakeTrailOverride`).
    pub field: String,
    /// The asset it was copied from (`Bagger/FrontTyre_Medium`, `Italjet_Dragster700Twin_2024`).
    pub source: String,
    /// Why that one.
    pub rule: String,
}

impl Borrowed {
    /// The manifest comment line (without the leading `# `).
    pub fn comment(&self) -> String {
        format!("choice: {} taken from {} ({}); the data leaves it unset", self.field, self.source, self.rule)
    }
}

/// The compounds of a tyre family, wettest first: a rain tyre with no strips of its own takes the nearest compound of the same
/// family that has them (the intermediate, then the road tyre, then the slicks).
pub const COMPOUNDS: [&str; 7] = ["Rain", "Wet", "Intermediate", "Medium", "Road", "Soft", "Hard"];

/// The compound of `candidates` nearest to `wanted` in [`COMPOUNDS`] (the first of the list on a tie, so the choice does not depend
/// on the order the folder lists its files in once the caller sorts them); `None` when none is known.
pub fn nearest_compound<'a>(wanted: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let place = |name: &str| COMPOUNDS.iter().position(|c| c.eq_ignore_ascii_case(name));
    let at = place(wanted)?;
    candidates
        .iter()
        .copied()
        .filter_map(|name| Some((place(name)?.abs_diff(at), name)))
        .min_by_key(|&(distance, name)| (distance, place(name)))
        .map(|(_, name)| name)
}

/// A vehicle that has a value to lend: its file's stem (`Italjet_Dragster700Twin_2024`) and its wheelbase in RIDE 6 centimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct Lender {
    pub name: String,
    pub wheelbase_cm: f64,
}

/// A vehicle's make, model words (letters and digits only, lower case) and year, from the stem of its physics file
/// (`Italjet_Dragster200CC_RM_2022`: `italjet`, `dragster200cc`, 2022; the variant tokens `RM` and `EM` are not model words).
pub fn parts_of(name: &str) -> (String, String, i32) {
    let mut words: Vec<&str> = name.split(['_', '-']).collect();
    let year = match words.last().and_then(|w| w.parse::<i32>().ok()).filter(|y| (1900..2100).contains(y)) {
        Some(year) => {
            words.pop();
            year
        }
        None => 0,
    };
    let make = words.first().map(|w| squash(w)).unwrap_or_default();
    let model: String =
        words.iter().skip(1).filter(|w| !["RM", "EM"].contains(w)).map(|w| squash(w)).collect::<Vec<_>>().concat();
    (make, model, year)
}

fn squash(text: &str) -> String {
    text.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
}

/// The vehicle to take a value from for `target`: one of the same make with the longest start of its model name in common (the
/// Dragster 200 CC takes the Dragster 700 Twin), then the nearest year; with none of the same make, the nearest wheelbase of any make
/// (the frame geometry of a bike of that size). The target itself is never its own lender.
pub fn nearest_lender<'a>(target: &Lender, lenders: &'a [Lender]) -> Option<(&'a Lender, &'static str)> {
    let (make, model, year) = parts_of(&target.name);
    let family = |lender: &&Lender| {
        let (_, other_model, other_year) = parts_of(&lender.name);
        let shared = model.chars().zip(other_model.chars()).take_while(|(a, b)| a == b).count();
        (shared, std::cmp::Reverse(year.abs_diff(other_year)), std::cmp::Reverse(lender.name.clone()))
    };
    let others = lenders.iter().filter(|l| l.name != target.name);
    let same_make = others.clone().filter(|l| parts_of(&l.name).0 == make).max_by_key(family);
    if let Some(lender) = same_make {
        return Some((lender, "the same make, the model whose name starts like this one's, then the nearest year"));
    }
    let nearest = others.min_by(|a, b| {
        let by = |l: &Lender| (l.wheelbase_cm - target.wheelbase_cm).abs();
        by(a).total_cmp(&by(b)).then_with(|| a.name.cmp(&b.name))
    })?;
    Some((nearest, "no model of the same make has it: the nearest wheelbase"))
}

#[cfg(test)]
#[path = "borrow_tests.rs"]
mod tests;

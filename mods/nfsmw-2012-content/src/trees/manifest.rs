//! The texts that say which solids of the 2005 world a set of the pack replaces: the replacement manifest the game reads
//! (`replace/<kind>-mw2012.toml`, `docs/modding/api-replace.md`) and the older `TREES/MW2012/profile.toml` (`docs/modding/effect-variants.md`,
//! "Profile format of the trees"), which a game without the replacement API reads for the trees. Both name the same two pack files
//! and the same swaps; a set differs in its kind (the option it is a value of), its families and its fit limits.

use super::build::{FOLDER, GEOMETRY_FILE, TEXTURES_FILE};
use super::species;
use super::stock::{self, Family};

/// The limits the game keeps a replacement to (`blackbox_modding::content::FitLimits`, the `[fit]` table): a copy is at most `spread`
/// times as wide as the footprint it replaces, is squeezed sideways to no less than `squeeze` of its height scale, and is not used
/// if it would be shorter than `min_height` of the replaced height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    pub spread: f32,
    pub squeeze: f32,
    pub min_height: f32,
}

/// The limits of the trees.
pub const TREE_LIMITS: Limits = Limits { spread: 1.25, squeeze: 0.5, min_height: 0.7 };

/// What a replacement set is called and holds.
pub struct Set<'a> {
    /// The option group (`trees` makes the option `trees_variant`) and what the option is called: a plain text, or the id of a message
    /// of the mod's catalogue.
    pub kind: &'a str,
    pub kind_label: &'a str,
    /// The value of the option that chooses the set, and its label.
    pub value: &'a str,
    pub value_label: &'a str,
    pub limits: Limits,
    pub families: Vec<&'static Family>,
}

/// The fit a family's solids ask for.
fn fit_of(family: &Family) -> &'static str {
    match family.row {
        true => "row",
        false => "height",
    }
}

/// The swaps of `families` for the species in `built` (the species and the number of levels each has): the game's solid, the pack's
/// model, and the fit, one for every solid of a family whose species was built.
pub fn swaps(families: &[&'static Family], built: &[(&str, usize)]) -> Vec<(String, String, &'static str)> {
    let mut out = Vec::new();
    for family in families {
        let Some(&(_, levels)) = built.iter().find(|(key, _)| *key == family.species) else { continue };
        for (solid, code) in family.names() {
            let model = species::solid_name(family.species, stock::level(code, family.row, levels));
            out.push((solid, model, fit_of(family)));
        }
    }
    out
}

/// The replacement manifest of `set`: the same swaps as [`profile_text`], declared through the one API. The game adds the value to
/// the kind's option from it, so the mod needs no code for the option.
pub fn replace_manifest(set: &Set<'_>, built: &[(&str, usize)]) -> String {
    let Set { kind, kind_label, value, value_label, limits, .. } = set;
    let Limits { spread, squeeze, min_height } = limits;
    let mut text = format!(
        "format = 1

[set]
kind = \"{kind}\"
kind_label = \"{kind_label}\"
value = \"{value}\"
value_label = \"{value_label}\"
fit = \"height\"

[files]
geometry = \"{FOLDER}/{GEOMETRY_FILE}\"
textures = \"{FOLDER}/{TEXTURES_FILE}\"

[fit]
spread = {spread}
squeeze = {squeeze}
min_height = {min_height}
"
    );
    for (solid, model, fit) in swaps(&set.families, built) {
        text.push_str(&format!("\n[[model]]\ntarget = \"{solid}\"\nmodel = \"{model}\"\nfit = \"{fit}\"\n"));
    }
    text
}

/// The text of the trees' `profile.toml`: a swap for every solid of the families of `set` whose species is in `built`.
pub fn profile_text(set: &Set<'_>, built: &[(&str, usize)]) -> String {
    let Limits { spread, squeeze, min_height } = set.limits;
    let mut text = format!(
        "format = 1\ngeometry = \"{FOLDER}/{GEOMETRY_FILE}\"\ntextures = \"{FOLDER}/{TEXTURES_FILE}\"\n\n[fit]\nspread = {spread}\nsqueeze = {squeeze}\nmin_height = {min_height}\n"
    );
    for (solid, model, fit) in swaps(&set.families, built) {
        text.push_str(&format!("\n[[swap]]\nsolid = \"{solid}\"\nmodel = \"{model}\"\nfit = \"{fit}\"\n"));
    }
    text
}

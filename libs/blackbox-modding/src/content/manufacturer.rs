//! A manufacturer a mod declares: a brand name and the logo the car screens show for it (`manufacturers/<id>.toml`).
//!
//! The game names a stock car's maker with a picture in the car select, the car lot and the garage. A mod car's brand is a
//! plain string, so without a manufacturer the screens draw the name themselves; a manufacturer gives it a picture. A mod
//! may also give a logo to a brand the game already has, to replace the game's own (`override`).

use serde::Deserialize;

use super::ContentError;

/// Longest manufacturer name, in characters.
const MAX_NAME_CHARS: usize = 32;
/// Longest logo path, in bytes.
const MAX_LOGO_BYTES: usize = 96;

/// A checked manufacturer manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manufacturer {
    /// The brand's name as shown. Brands are told apart by it, ignoring case, spaces and punctuation.
    pub name: String,
    /// The logo: a PNG under the mod's `files/` folder, a relative path of letters, digits, `_ - . /` ending in `.png`.
    pub logo: String,
    /// Whether the logo replaces the one of an existing manufacturer of that name (the game's or another mod's).
    pub overriding: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    name: String,
    logo: String,
    #[serde(default, rename = "override")]
    overriding: bool,
}

/// Parse the text of the manufacturer manifest `id` (the file name without `.toml`).
pub fn parse(id: &str, text: &str) -> Result<Manufacturer, ContentError> {
    if !super::is_id(id) {
        return Err(ContentError::InvalidId(id.to_owned()));
    }
    let raw: Raw = toml::from_str(text).map_err(|error| ContentError::Parse(error.to_string()))?;
    let name = raw.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS || name.chars().any(char::is_control) {
        return Err(ContentError::InvalidName(raw.name));
    }
    if !is_logo_path(&raw.logo) {
        return Err(ContentError::Field {
            field: "logo",
            reason: "a logo is a relative path under files/ of letters, digits, _ - . and /, ending in .png",
        });
    }
    Ok(Manufacturer { name: name.to_owned(), logo: raw.logo, overriding: raw.overriding })
}

/// A relative `.png` path of letters, digits and `_ - . /`, with no empty or `..` part.
fn is_logo_path(path: &str) -> bool {
    let chars_ok = path.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"_-./".contains(&byte));
    let parts_ok = path.split('/').all(|part| !part.is_empty() && part != "." && part != "..");
    chars_ok && parts_ok && path.len() <= MAX_LOGO_BYTES && path.to_ascii_lowercase().ends_with(".png")
}

/// A brand name reduced for comparison: upper case letters and digits only, so `Aston Martin`, `ASTON_MARTIN` and
/// `Aston-Martin` are one brand.
pub fn brand_key(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_uppercase).collect()
}

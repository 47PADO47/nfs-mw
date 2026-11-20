//! The `world` setting: the id of the second world to play instead of the city (`docs/specs/second-world.md`).
//! Unset, or `rockport`, is the city.

use std::fmt;
use std::str::FromStr;

/// The longest id kept; a world id is a file name of a mod (`worlds/<id>.toml`), at most 32 bytes.
const MAX: usize = 32;

/// The name that means the city, in the menu and on the command line.
pub const CITY: &str = "rockport";

/// A world id, stored inline so the settings stay `Copy`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldId {
    bytes: [u8; MAX],
    len: u8,
}

impl WorldId {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or(CITY)
    }
}

impl FromStr for WorldId {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        let valid = !text.is_empty()
            && text.len() <= MAX
            && text.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-');
        if !valid {
            return Err(format!("{text:?} is not a world id (letters, digits, _ and -, 1 to {MAX} bytes)"));
        }
        let mut bytes = [0; MAX];
        bytes[..text.len()].copy_from_slice(text.as_bytes());
        Ok(Self { bytes, len: text.len() as u8 })
    }
}

impl fmt::Display for WorldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for WorldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WorldId({})", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_and_the_city_is_the_default_name() {
        let id: WorldId = "okayama".parse().unwrap();
        assert_eq!(id.to_string(), "okayama");
        assert_eq!("Rockport".parse::<WorldId>().unwrap().to_string().to_lowercase(), CITY);
    }

    #[test]
    fn a_bad_id_is_refused() {
        assert!("".parse::<WorldId>().is_err());
        assert!("a b".parse::<WorldId>().is_err());
        assert!("x".repeat(33).parse::<WorldId>().is_err());
    }
}

//! A hash-to-name dictionary.
//!
//! Files store only hashes. [`crate::Database`] fills its dictionary with every string it finds
//! (vault names, dependency names, the `StrE` string table, type names, string values); names
//! that only exist in code (most class and field names) can be added from an external list with
//! [`Names::add_lines`].

use std::collections::HashMap;

use crate::hash::vlt_hash;

#[derive(Debug, Clone, Default)]
pub struct Names {
    map: HashMap<u32, String>,
}

impl Names {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `name` under its hash and returns the hash. An existing name for the hash is kept.
    pub fn insert(&mut self, name: &str) -> u32 {
        let key = vlt_hash(name);
        self.map.entry(key).or_insert_with(|| name.to_owned());
        key
    }

    /// Adds one name per line and returns how many lines held a name. Blank lines and lines
    /// starting with `#` are skipped. Lines in the `0x1234abcd - name` form (the decomp's
    /// `symbols/vlt.txt`) contribute `name`; the listed hash is ignored because the hash is
    /// recomputed.
    pub fn add_lines(&mut self, text: &str) -> usize {
        let mut added = 0;
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let name = match line.split_once(" - ") {
                Some((hash, name)) if hash.starts_with("0x") => name,
                _ => line,
            };
            self.insert(name);
            added += 1;
        }
        added
    }

    pub fn get(&self, key: u32) -> Option<&str> {
        self.map.get(&key).map(String::as_str)
    }

    /// The name of `key`, or `0x1234ABCD` when it is unknown.
    pub fn display(&self, key: u32) -> String {
        self.get(key).map_or_else(|| format!("0x{key:08X}"), str::to_owned)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &str)> {
        self.map.iter().map(|(&k, v)| (k, v.as_str()))
    }
}

impl<S: AsRef<str>> Extend<S> for Names {
    fn extend<I: IntoIterator<Item = S>>(&mut self, iter: I) {
        for name in iter {
            self.insert(name.as_ref());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_lists() {
        let mut names = Names::new();
        assert_eq!(names.insert("pvehicle"), 0x4A97_EC8F);
        assert_eq!(names.get(0x4A97_EC8F), Some("pvehicle"));
        assert_eq!(names.display(1), "0x00000001");

        let added = names.add_lines("# comment\n\n0x966d915e - _Array_Amplitude\r\nchassis\n");
        assert_eq!(added, 2);
        assert_eq!(names.get(0x966D_915E), Some("_Array_Amplitude"));
        assert_eq!(names.get(vlt_hash("chassis")), Some("chassis"));

        names.extend(["engine", "tires"]);
        assert_eq!(names.len(), 5);
    }
}

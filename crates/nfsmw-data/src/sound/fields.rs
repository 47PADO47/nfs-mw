//! Typed reads from a collection with the game's defaults for a missing field, and the raw record types
//! the reader leaves as bytes (matrices, `stShiftPair`, `UpgradeSpecs`).

use blackbox_attrib::{CollectionRef, RefSpec, Value};

/// A collection read by field name; a missing field reads as zero or empty.
#[derive(Clone, Copy)]
pub(super) struct Fields<'a>(pub CollectionRef<'a>);

impl<'a> Fields<'a> {
    pub fn f32(&self, name: &str) -> f32 {
        self.0.get_f32(name).unwrap_or(0.0)
    }

    pub fn u32(&self, name: &str) -> u32 {
        self.0.get_u32(name).unwrap_or(0)
    }

    pub fn i32(&self, name: &str) -> i32 {
        self.0.get_i32(name).unwrap_or(0)
    }

    pub fn bool(&self, name: &str) -> bool {
        self.0.get_bool(name).unwrap_or(false)
    }

    pub fn string(&self, name: &str) -> String {
        self.0.get_str(name).unwrap_or_default().to_owned()
    }

    /// Every string of an array field (a scalar is a one-item array); empty strings are dropped.
    pub fn strings(&self, name: &str) -> Vec<String> {
        let items: Vec<&Value> = match self.0.get(name) {
            Some(Value::Array(items)) => items.iter().collect(),
            Some(one) => vec![one],
            None => Vec::new(),
        };
        items.into_iter().filter_map(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned).collect()
    }

    /// The raw bytes of every item of an array field (a scalar is a one-item array).
    pub fn raw_items(&self, name: &str) -> Vec<&'a [u8]> {
        let items: Vec<&Value> = match self.0.get(name) {
            Some(Value::Array(items)) => items.iter().collect(),
            Some(one) => vec![one],
            None => Vec::new(),
        };
        items
            .into_iter()
            .filter_map(|v| match v {
                Value::Raw { bytes, .. } => Some(bytes.as_slice()),
                _ => None,
            })
            .collect()
    }

    /// The collection name, or `0x1234ABCD` when the database does not know it.
    pub fn name(&self) -> String {
        self.0.name().map_or_else(|| format!("0x{:08X}", self.0.key()), str::to_owned)
    }
}

/// A 4x4 float matrix whose rows are Bezier control points `(x, y, 0, 0)`: the `(x, y)` of each row.
pub(super) fn bezier_points(bytes: &[u8]) -> Option<[[f32; 2]; 4]> {
    if bytes.len() != 64 {
        return None;
    }
    let float = |i: usize| f32::from_le_bytes([bytes[i * 4], bytes[i * 4 + 1], bytes[i * 4 + 2], bytes[i * 4 + 3]]);
    Some(std::array::from_fn(|row| [float(row * 4), float(row * 4 + 1)]))
}

/// A `stShiftPair`: `{i16 RPM, i16 Time}`.
pub(super) fn shift_pair(bytes: &[u8]) -> Option<(i32, i32)> {
    let [a, b, c, d] = <[u8; 4]>::try_from(bytes).ok()?;
    Some((i32::from(i16::from_le_bytes([a, b])), i32::from(i16::from_le_bytes([c, d]))))
}

/// An `UpgradeSpecs`: a RefSpec `{class, collection, 0}` and a `u8` level (16 bytes).
pub(super) fn upgrade_spec(bytes: &[u8]) -> Option<(RefSpec, u8)> {
    if bytes.len() != 16 {
        return None;
    }
    let word = |at: usize| u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    Some((RefSpec { class: word(0), collection: word(4) }, bytes[12]))
}

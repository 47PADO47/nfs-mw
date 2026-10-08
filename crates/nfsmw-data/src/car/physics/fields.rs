//! Typed reads from an AttribSys collection, with the game's defaults for a missing field (zero).

use blackbox_attrib::{CollectionRef, Value};
use glam::Vec3;

/// A collection whose fields are read by name.
#[derive(Clone, Copy)]
pub struct Fields<'a>(pub CollectionRef<'a>);

impl<'a> Fields<'a> {
    pub fn f32(&self, name: &str) -> f32 {
        self.0.get_f32(name).unwrap_or(0.0)
    }

    pub fn bool(&self, name: &str) -> bool {
        self.0.get_bool(name).unwrap_or(false)
    }

    /// Every float of an array field (a scalar is a one-item array).
    pub fn floats(&self, name: &str) -> Vec<f32> {
        match self.0.get(name) {
            Some(Value::Array(items)) => items.iter().filter_map(Value::as_f32).collect(),
            Some(v) => v.as_f32().into_iter().collect(),
            None => Vec::new(),
        }
    }

    /// The first two items of a float array, or the one scalar twice: front and rear, or low and high.
    pub fn pair(&self, name: &str) -> [f32; 2] {
        match self.floats(name).as_slice() {
            [] => [0.0; 2],
            [one] => [*one; 2],
            [a, b, ..] => [*a, *b],
        }
    }

    /// An `AxlePair` field (front, rear): stored as two floats in a record the reader leaves raw.
    pub fn axle_pair(&self, name: &str) -> [f32; 2] {
        let value = self.0.get_at(name, 0);
        match value {
            Some(Value::Raw { bytes, .. }) if bytes.len() == 8 => [
                f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                f32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            ],
            _ => self.pair(name),
        }
    }

    /// The xyz of a vector field.
    pub fn vec3(&self, name: &str) -> Vec3 {
        self.0.get_vector4(name).map_or(Vec3::ZERO, |v| Vec3::new(v[0], v[1], v[2]))
    }

    /// The collection a link field refers to (item 0 of an array of links).
    pub fn link(&self, name: &str) -> Option<Fields<'a>> {
        self.0.follow(name).map(Fields)
    }
}

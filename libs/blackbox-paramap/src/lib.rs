//! Spatial parameter-map layers. Spec: `spatial-parameter-maps.md`.
//! Bytes in, validated typed spatial queries out; no filesystem access.

mod parse;

#[cfg(test)]
mod tests;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("invalid spatial parameter map: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Chunk(#[from] blackbox_chunk::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A typed spatial layer with an optional quadrant tree.
#[derive(Debug, Clone)]
pub struct Layer {
    pub name_hash: u32,
    pub bounds: [f32; 4],
    fields: Vec<(i32, usize)>,
    stride: usize,
    sets: usize,
    data: Vec<u8>,
    nodes: Vec<[usize; 4]>,
}

impl Layer {
    pub fn read_all(bytes: &[u8]) -> Result<Vec<Self>> {
        parse::read_all(bytes)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn set_count(&self) -> usize {
        self.sets
    }

    /// Original quadrant selection, including its unusual reversed-bound guard.
    pub fn set_at(&self, x: f32, y: f32) -> Option<usize> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let [mut left, mut top, mut right, mut bottom] = self.bounds;
        if x < left && x > right && y < top && y > bottom {
            return Some(0);
        }
        if self.nodes.is_empty() {
            return Some(0);
        }
        let mut index = 0;
        for _ in 0..self.nodes.len() {
            let node = self.nodes.get(index)?;
            if node[0] == 0 {
                return Some(node[1]);
            }
            let centre_x = (left + right) * 0.5;
            let centre_y = (top + bottom) * 0.5;
            let upper_x = x >= centre_x;
            let upper_y = y >= centre_y;
            index = node[usize::from(upper_x) * 2 + usize::from(upper_y)];
            match upper_x {
                true => left = centre_x,
                false => right = centre_x,
            }
            match upper_y {
                true => top = centre_y,
                false => bottom = centre_y,
            }
        }
        None
    }

    /// The integer field of a selected set; mismatched types return `None`.
    pub fn integer(&self, set: usize, field: usize) -> Option<i32> {
        Some(i32::from_le_bytes(self.field(set, field, 1)?))
    }

    /// The float field of a selected set; mismatched types return `None`.
    pub fn float(&self, set: usize, field: usize) -> Option<f32> {
        Some(f32::from_le_bytes(self.field(set, field, 0)?))
    }

    fn field(&self, set: usize, field: usize, expected: i32) -> Option<[u8; 4]> {
        if set >= self.sets {
            return None;
        }
        let &(kind, offset) = self.fields.get(field)?;
        if kind != expected {
            return None;
        }
        let at = set.checked_mul(self.stride)?.checked_add(offset)?;
        self.data.get(at..at + 4)?.try_into().ok()
    }
}

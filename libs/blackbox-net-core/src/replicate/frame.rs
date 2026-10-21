//! One tick of entities: a list sorted by id, stored flat so that clearing it keeps its capacity.

use super::ReplicateError;
use super::schema::{KindId, Schema, fits};

/// One entity in a frame. Its values are `values[start .. start + len]`.
#[derive(Debug, Clone, Copy)]
struct Entry {
    id: u16,
    kind: KindId,
    start: u32,
    len: u8,
}

/// One tick of entities, sorted by id. The client's view of the world is also a `Frame`.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    tick: u32,
    entries: Vec<Entry>,
    values: Vec<u32>,
}

impl Frame {
    /// An empty frame at tick 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// The tick this frame holds.
    pub fn tick(&self) -> u32 {
        self.tick
    }

    /// How many entities the frame holds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the frame holds no entities.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The kind and field values of entity `id`, found by binary search.
    pub fn get(&self, id: u16) -> Option<(KindId, &[u32])> {
        let index = self.entries.binary_search_by_key(&id, |entry| entry.id).ok()?;
        let entry = self.entries.get(index)?;
        Some((entry.kind, self.values_of(entry)))
    }

    /// Every entity in id order: `(id, kind, values)`.
    pub fn iter(&self) -> impl Iterator<Item = (u16, KindId, &[u32])> + '_ {
        self.entries.iter().map(move |entry| (entry.id, entry.kind, self.values_of(entry)))
    }

    /// Empties the frame and sets its tick. Keeps the capacity of both lists.
    pub fn clear(&mut self, tick: u32) {
        self.tick = tick;
        self.entries.clear();
        self.values.clear();
    }

    /// Appends entity `id` of `kind` with its field `values`.
    ///
    /// The id must be above the last one, else [`ReplicateError::BadOrder`]. An unknown kind is
    /// [`ReplicateError::UnknownKind`], a wrong value count [`ReplicateError::BadSchema`], and a value that does not fit
    /// its field [`ReplicateError::BadValue`]. A failed push changes nothing.
    pub fn push(&mut self, schema: &Schema, id: u16, kind: KindId, values: &[u32]) -> Result<(), ReplicateError> {
        if self.entries.last().is_some_and(|last| id <= last.id) {
            return Err(ReplicateError::BadOrder);
        }
        let fields = schema.fields(kind).ok_or(ReplicateError::UnknownKind)?;
        if fields.len() != values.len() {
            return Err(ReplicateError::BadSchema);
        }
        if !fields.iter().zip(values).all(|(field, &value)| fits(value, field.bits)) {
            return Err(ReplicateError::BadValue);
        }
        let start = u32::try_from(self.values.len()).map_err(|_| ReplicateError::BadSchema)?;
        let len = u8::try_from(fields.len()).map_err(|_| ReplicateError::BadSchema)?;
        self.entries.push(Entry { id, kind, start, len });
        self.values.extend_from_slice(values);
        Ok(())
    }

    /// The field values of `entry`. The bounds always hold for entries this frame made.
    fn values_of(&self, entry: &Entry) -> &[u32] {
        let start = entry.start as usize;
        self.values.get(start..start.saturating_add(usize::from(entry.len))).unwrap_or(&[])
    }
}

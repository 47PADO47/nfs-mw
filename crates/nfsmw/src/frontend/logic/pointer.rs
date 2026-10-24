//! Which of a screen's objects the mouse is on. Screens name their clickable objects (arrows, slots, buttons) in lists,
//! and this answers with the place in the list, so a click handler is a `match` on an index and not a search.

use blackbox_feng::ObjectRef;
use glam::Vec2;

use super::Cx;

impl Cx<'_> {
    /// The index in `objects` of the one under `point`. A gap, an object the package does not have, is never hit.
    pub fn hit_index(&self, objects: &[Option<ObjectRef>], point: Vec2) -> Option<usize> {
        let present: Vec<ObjectRef> = objects.iter().flatten().copied().collect();
        let hit = self.rt.object_at(&present, point)?;
        objects.iter().position(|object| *object == Some(hit))
    }
}

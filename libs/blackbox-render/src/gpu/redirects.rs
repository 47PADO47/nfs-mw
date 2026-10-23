//! Texture redirects (animated textures): a slot-indexed table, so the per-draw lookup in every pass is an
//! array read instead of a hash.

/// Texture slot -> the slot drawn in its place. Slots are small dense indices, so the table is a vector
/// indexed by slot; `None` means no redirect.
#[derive(Default)]
pub(super) struct Redirects {
    to: Vec<Option<usize>>,
}

impl Redirects {
    /// The slot drawn in place of `from`, when it has a redirect.
    pub(super) fn get(&self, from: &usize) -> Option<&usize> {
        self.to.get(*from)?.as_ref()
    }

    /// The slot to draw for `slot`: its redirect when it has one, else `slot` itself.
    pub(super) fn resolve(&self, slot: usize) -> usize {
        self.to.get(slot).copied().flatten().unwrap_or(slot)
    }

    /// Draw `to` wherever `from` is used.
    pub(super) fn insert(&mut self, from: usize, to: usize) {
        if self.to.len() <= from {
            self.to.resize(from + 1, None);
        }
        self.to[from] = Some(to);
    }

    /// Restore `from`.
    pub(super) fn remove(&mut self, from: usize) {
        let Some(entry) = self.to.get_mut(from) else { return };
        *entry = None;
    }

    /// Drop every redirect from or to `slot`, for a texture that was freed.
    pub(super) fn forget(&mut self, slot: usize) {
        for (from, to) in self.to.iter_mut().enumerate() {
            if from == slot || *to == Some(slot) {
                *to = None;
            }
        }
    }

    /// Whether no slot is redirected.
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.to.iter().all(Option::is_none)
    }
}

#[cfg(test)]
mod tests {
    use super::Redirects;

    #[test]
    fn resolve_follows_a_redirect_and_leaves_other_slots_alone() {
        let mut redirects = Redirects::default();
        assert_eq!(redirects.resolve(4), 4);
        redirects.insert(2, 7);
        assert_eq!(redirects.resolve(2), 7);
        assert_eq!(redirects.get(&2), Some(&7));
        assert_eq!(redirects.resolve(1), 1);
        assert_eq!(redirects.resolve(9), 9);
        assert_eq!(redirects.get(&9), None);
    }

    #[test]
    fn remove_restores_the_slot() {
        let mut redirects = Redirects::default();
        redirects.insert(3, 1);
        redirects.remove(3);
        redirects.remove(40);
        assert_eq!(redirects.resolve(3), 3);
        assert!(redirects.is_empty());
    }

    #[test]
    fn forget_drops_redirects_from_and_to_a_freed_slot() {
        let mut redirects = Redirects::default();
        redirects.insert(2, 5);
        redirects.insert(5, 1);
        redirects.insert(3, 4);
        redirects.forget(5);
        assert_eq!(redirects.resolve(2), 2);
        assert_eq!(redirects.resolve(5), 5);
        assert_eq!(redirects.resolve(3), 4);
    }
}

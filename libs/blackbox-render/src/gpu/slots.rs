//! A vector of optional resources with slot reuse, so handles stay small and
//! streamed-out resources can be freed.

pub(super) struct Slots<T> {
    items: Vec<Option<T>>,
    free: Vec<usize>,
}

impl<T> Slots<T> {
    pub(super) fn new() -> Self {
        Self { items: Vec::new(), free: Vec::new() }
    }

    pub(super) fn insert(&mut self, item: T) -> usize {
        match self.free.pop() {
            Some(i) => {
                self.items[i] = Some(item);
                i
            }
            None => {
                self.items.push(Some(item));
                self.items.len() - 1
            }
        }
    }

    pub(super) fn remove(&mut self, index: usize) -> Option<T> {
        let item = self.items.get_mut(index)?.take();
        if item.is_some() {
            self.free.push(index);
        }
        item
    }

    pub(super) fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)?.as_ref()
    }

    pub(super) fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)?.as_mut()
    }

    pub(super) fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.items.iter_mut().filter_map(Option::as_mut)
    }

    /// The live items with their slot numbers, in slot order.
    pub(super) fn iter_indexed(&self) -> impl Iterator<Item = (usize, &T)> + Clone {
        self.items.iter().enumerate().filter_map(|(i, item)| Some((i, item.as_ref()?)))
    }

    /// Number of live items.
    pub(super) fn len(&self) -> usize {
        self.items.len() - self.free.len()
    }
}

#[cfg(test)]
mod tests {
    use super::Slots;

    #[test]
    fn slots_are_reused() {
        let mut s = Slots::new();
        let a = s.insert("a");
        let b = s.insert("b");
        assert_eq!(s.remove(a), Some("a"));
        assert_eq!(s.remove(a), None);
        assert_eq!(s.insert("c"), a);
        assert_eq!((s.get(a), s.get(b), s.len()), (Some(&"c"), Some(&"b"), 2));
    }

    #[test]
    fn indexed_iteration_skips_freed_slots() {
        let mut s = Slots::new();
        let a = s.insert("a");
        s.insert("b");
        s.insert("c");
        s.remove(a);
        assert_eq!(s.iter_indexed().collect::<Vec<_>>(), [(1, &"b"), (2, &"c")]);
    }
}

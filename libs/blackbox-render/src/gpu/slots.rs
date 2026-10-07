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
}

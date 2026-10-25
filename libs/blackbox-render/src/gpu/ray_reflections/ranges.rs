//! Reuse vacated geometry spans without moving still-live mesh addresses.
use std::ops::Range;
#[derive(Default)]
pub(super) struct Ranges {
    free: Vec<Range<usize>>,
    pub end: usize,
}
impl Ranges {
    pub fn fits(&self, count: usize, limit: usize) -> bool {
        self.free.iter().any(|r| r.len() >= count) || self.end.saturating_add(count) <= limit
    }
    pub fn allocate(&mut self, count: usize) -> Range<usize> {
        if let Some(index) = self.free.iter().position(|r| r.len() >= count) {
            let start = self.free[index].start;
            self.free[index].start += count;
            if self.free[index].is_empty() {
                self.free.remove(index);
            }
            return start..start + count;
        }
        let start = self.end;
        self.end += count;
        start..self.end
    }
    pub fn release(&mut self, range: Range<usize>) {
        if range.is_empty() {
            return;
        }
        self.free.push(range);
        self.free.sort_unstable_by_key(|r| r.start);
        let mut merged: Vec<Range<usize>> = Vec::new();
        for range in self.free.drain(..) {
            if let Some(last) = merged.last_mut()
                && last.end == range.start
            {
                last.end = range.end;
                continue;
            }
            merged.push(range);
        }
        while merged.last().is_some_and(|r| r.end == self.end) {
            self.end = merged.pop().unwrap().start;
        }
        self.free = merged;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn departing_meshes_reuse_holes_without_moving_live_spans() {
        let mut a = Ranges::default();
        let first = a.allocate(8);
        let live = a.allocate(9);
        let last = a.allocate(4);
        a.release(first);
        assert_eq!(a.allocate(5), 0..5);
        assert_eq!(live, 8..17);
        a.release(last);
        assert_eq!(a.end, 17);
        a.release(5..8);
        a.release(0..5);
        assert!(a.fits(8, 17));
        assert!(!a.fits(10, 17));
        assert_eq!(a.allocate(8), 0..8);
    }
}

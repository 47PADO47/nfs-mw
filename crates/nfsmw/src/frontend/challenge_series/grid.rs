//! The icon grid's selection: a window of `width` x `height` slots over a list, moved a column or a row at a time. It
//! follows the original array scroller (`ArrayScroller::ScrollHor`, `ScrollVer`, `ForceSelectionOnScreen`,
//! `UpdateScrollbar`), where left and right wrap inside a row and up and down stop at the ends.

/// Where the selection and the window are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    width: usize,
    height: usize,
    count: usize,
    /// The index selected.
    pub current: usize,
    /// The index of the slot at the top left of the window (a multiple of the width).
    pub start: usize,
}

/// Which way a step goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Left,
    Right,
    Up,
    Down,
}

/// What the scrollbar shows (`FEScrollBar::Update`): rows in the window, rows in the list and the window's first row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bar {
    pub view_rows: usize,
    pub rows: usize,
    pub top_row: usize,
}

impl Grid {
    pub fn new(width: usize, height: usize, count: usize) -> Self {
        Self { width, height, count, current: 0, start: 0 }
    }

    /// Selects `index` (clamped to the list) with the window the original's `SetInitialPosition` gives: the first
    /// window when it shows the index, else the one that has its row last.
    pub fn select(&mut self, index: usize) {
        if self.count == 0 {
            return;
        }
        self.current = index.min(self.count - 1);
        let size = self.width * self.height;
        self.start = match self.current < size {
            true => 0,
            false => (self.current / self.width + 1).saturating_sub(self.height) * self.width,
        };
    }

    /// The slot of the window the selection is in, or none when it is outside (it never is after a step).
    pub fn slot(&self) -> Option<usize> {
        let slot = self.current.checked_sub(self.start)?;
        (slot < self.width * self.height).then_some(slot)
    }

    /// Selects the item a slot of the window shows, leaving the window where it is. Returns whether the slot has one.
    pub fn pick(&mut self, slot: usize) -> bool {
        let Some(index) = self.index_of(slot) else { return false };
        self.current = index;
        true
    }

    /// The list index a slot of the window shows, or none past the end of the list.
    pub fn index_of(&self, slot: usize) -> Option<usize> {
        let index = self.start + slot;
        (index < self.count).then_some(index)
    }

    /// Moves the selection one step. Returns whether it changed.
    pub fn step(&mut self, step: Step) -> bool {
        if self.count == 0 {
            return false;
        }
        let (target, start) = match step {
            Step::Left | Step::Right => (Some(self.across(step)), self.start),
            Step::Up => self.up(),
            Step::Down => self.down(),
        };
        let Some(target) = target.filter(|t| *t != self.current) else { return false };
        self.start = self.on_screen(target, start);
        self.current = target;
        true
    }

    /// The scrollbar's numbers for the window.
    pub fn bar(&self) -> Bar {
        Bar {
            view_rows: self.height,
            rows: self.count.div_ceil(self.width.max(1)),
            top_row: self.start / self.width + 1,
        }
    }

    /// Left and right wrap inside the row; the last row wraps over the items it has.
    fn across(&self, step: Step) -> usize {
        let (w, n, cur) = (self.width, self.count, self.current);
        match step {
            Step::Left if cur % w == 0 => (cur + w - 1).min(n - 1),
            Step::Left => cur - 1,
            _ if (cur + 1) % w == 0 => cur + 1 - w,
            _ if cur + 1 >= n => cur / w * w,
            _ => cur + 1,
        }
    }

    /// One row up, and the window's start: the window moves up when the target is above it. The top row stays.
    fn up(&self) -> (Option<usize>, usize) {
        let Some(target) = self.current.checked_sub(self.width) else { return (None, self.start) };
        let start = if target < self.start { self.start.saturating_sub(self.width) } else { self.start };
        (Some(target), start)
    }

    /// One row down, and the window's start: from a row that is not the last, a short last row is reached at its last
    /// item; the window moves down when the target is past its slots. The last row stays.
    fn down(&self) -> (Option<usize>, usize) {
        let (w, n, cur) = (self.width, self.count, self.current);
        let mut target = cur + w;
        if target >= n {
            let last_row = (n - 1) / w;
            if cur / w >= last_row {
                return (None, self.start);
            }
            target = n - 1;
        }
        let start = if self.start + w * self.height <= target { self.start + w } else { self.start };
        (Some(target), start)
    }

    /// `ForceSelectionOnScreen`: the window start that shows `target`, given the start the step asked for.
    fn on_screen(&self, target: usize, start: usize) -> usize {
        let (w, h) = (self.width, self.height);
        if target < start {
            return target / w * w;
        }
        if target >= start + w * h {
            return (target / w + 1).saturating_sub(h) * w;
        }
        start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(count: usize) -> Grid {
        Grid::new(4, 3, count)
    }

    #[test]
    fn left_and_right_wrap_inside_a_row() {
        let mut g = grid(70);
        assert!(g.step(Step::Left));
        assert_eq!(g.current, 3, "left from the first item goes to the end of the row");
        assert!(g.step(Step::Right));
        assert_eq!(g.current, 0);
        g.select(2);
        g.step(Step::Right);
        g.step(Step::Right);
        assert_eq!(g.current, 0, "the last item of a row wraps to its first");
    }

    #[test]
    fn the_last_row_wraps_over_the_items_it_has() {
        let mut g = grid(70);
        g.select(69);
        g.step(Step::Right);
        assert_eq!(g.current, 68);
        g.step(Step::Left);
        assert_eq!(g.current, 69, "left from the row's first item goes to its last item");
        let mut short = grid(6);
        short.select(4);
        short.step(Step::Left);
        assert_eq!(short.current, 5, "a short last row wraps to its own end");
    }

    #[test]
    fn down_scrolls_a_row_when_the_selection_leaves_the_window() {
        let mut g = grid(70);
        for _ in 0..2 {
            assert!(g.step(Step::Down));
        }
        assert_eq!((g.current, g.start), (8, 0));
        assert!(g.step(Step::Down));
        assert_eq!((g.current, g.start), (12, 4), "the fourth row pushes the window down one row");
        assert_eq!(g.slot(), Some(8));
    }

    #[test]
    fn up_scrolls_back_and_stops_at_the_top() {
        let mut g = grid(70);
        g.select(12);
        assert_eq!(g.start, 4);
        g.step(Step::Up);
        g.step(Step::Up);
        g.step(Step::Up);
        assert_eq!((g.current, g.start), (0, 0));
        assert!(!g.step(Step::Up), "the top row stays");
    }

    #[test]
    fn down_into_a_short_last_row_lands_on_its_last_item() {
        let mut g = grid(70);
        g.select(66);
        assert!(g.step(Step::Down));
        assert_eq!(g.current, 69, "66 + 4 is past the end: the last item");
        assert!(!g.step(Step::Down), "the last row stays");
    }

    #[test]
    fn selecting_far_down_puts_that_row_last_in_the_window() {
        let mut g = grid(70);
        g.select(69);
        assert_eq!(g.start, 60);
        assert_eq!(g.slot(), Some(9));
        assert_eq!(g.index_of(9), Some(69));
        assert_eq!(g.index_of(10), None, "the list ends mid-window");
    }

    #[test]
    fn the_scrollbar_counts_rows() {
        let mut g = grid(70);
        assert_eq!(g.bar(), Bar { view_rows: 3, rows: 18, top_row: 1 });
        g.select(40);
        assert_eq!(g.bar().top_row, 9);
    }

    #[test]
    fn an_empty_list_does_nothing() {
        let mut g = grid(0);
        assert!(!g.step(Step::Right));
        g.select(3);
        assert_eq!(g.current, 0);
    }
}

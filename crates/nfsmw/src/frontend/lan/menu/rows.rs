//! What a LAN page shows: rows of a label and a value, the selection, a side column and a footer. The pages build it each
//! frame; `view.rs` draws it and the tests read it. It names no layout and no colour.

/// What a row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// Accept does something.
    Action,
    /// Left and right cycle the value.
    Choice,
    /// Accept starts typing into the value; Accept or Back ends it.
    Text { editing: bool },
    /// Shown only (a player of the roster, a result line); Accept may open something.
    Info,
}

/// One row of a page.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub label: String,
    pub value: String,
    pub enabled: bool,
    pub kind: RowKind,
}

impl Row {
    pub fn action(label: impl Into<String>, enabled: bool) -> Self {
        Self { label: label.into(), value: String::new(), enabled, kind: RowKind::Action }
    }

    pub fn choice(label: impl Into<String>, value: impl Into<String>, enabled: bool) -> Self {
        Self { label: label.into(), value: value.into(), enabled, kind: RowKind::Choice }
    }

    pub fn text(label: impl Into<String>, value: impl Into<String>, editing: bool) -> Self {
        Self { label: label.into(), value: value.into(), enabled: true, kind: RowKind::Text { editing } }
    }

    pub fn info(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self { label: label.into(), value: value.into(), enabled: true, kind: RowKind::Info }
    }
}

/// A footer hint: the key and what it does.
pub type Hint = (&'static str, &'static str);

/// A page as it is drawn.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct PageView {
    pub title: String,
    /// A line under the title: the status, the room's mode, a countdown.
    pub subtitle: String,
    pub rows: Vec<Row>,
    pub selected: usize,
    /// The side column's heading and lines (the chat of the lobby), oldest first.
    pub side_title: String,
    pub side: Vec<String>,
    /// A line in the warning colour: why something failed.
    pub error: Option<String>,
    pub hints: Vec<Hint>,
}

impl PageView {
    /// The row whose label is `label`, for the tests.
    #[cfg(test)]
    pub fn row(&self, label: &str) -> Option<&Row> {
        self.rows.iter().find(|row| row.label == label)
    }

    /// The labels of the rows, for the tests.
    #[cfg(test)]
    pub fn labels(&self) -> Vec<&str> {
        self.rows.iter().map(|row| row.label.as_str()).collect()
    }
}

/// The selected row of a list of `len` rows: moves wrap, and a list that shrank keeps the selection inside it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cursor(pub usize);

impl Cursor {
    pub fn up(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.0 = (self.clamped(len) + len - 1) % len;
    }

    pub fn down(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.0 = (self.clamped(len) + 1) % len;
    }

    /// The selection inside a list of `len` rows.
    pub fn clamped(self, len: usize) -> usize {
        self.0.min(len.saturating_sub(1))
    }
}

/// The value after `current` in `values` (`forward`) or before it, wrapping; the first value when `current` is not one.
pub fn cycle<T: PartialEq + Copy>(values: &[T], current: T, forward: bool) -> T {
    let Some(at) = values.iter().position(|v| *v == current) else {
        return values.first().copied().unwrap_or(current);
    };
    let len = values.len();
    match forward {
        true => values[(at + 1) % len],
        false => values[(at + len - 1) % len],
    }
}

/// `value` moved by `step` (signed), kept inside `min..=max`.
pub fn nudge(value: u8, step: i16, min: u8, max: u8) -> u8 {
    let moved = (i16::from(value) + step).clamp(i16::from(min), i16::from(max));
    moved as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_wraps_and_stays_inside() {
        let mut cursor = Cursor(0);
        cursor.up(3);
        assert_eq!(cursor, Cursor(2));
        cursor.down(3);
        assert_eq!(cursor, Cursor(0));
        assert_eq!(Cursor(9).clamped(4), 3);
        cursor.down(0);
        assert_eq!(cursor, Cursor(0));
    }

    #[test]
    fn values_cycle_and_numbers_stay_in_range() {
        assert_eq!(cycle(&[1, 2, 3], 3, true), 1);
        assert_eq!(cycle(&[1, 2, 3], 1, false), 3);
        assert_eq!(cycle(&[1, 2, 3], 7, true), 1);
        assert_eq!(nudge(95, 10, 0, 100), 100);
        assert_eq!(nudge(2, -5, 0, 5), 0);
    }
}

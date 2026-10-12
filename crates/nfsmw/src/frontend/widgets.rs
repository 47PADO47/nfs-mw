//! Two option widgets the menus share, as plain state with no drawing in them: a [`Selector`] (the choice between
//! options that the arrows at the top of a controls screen change, such as the device) and a [`DualValue`] (a row
//! with a primary and a secondary value, one per column, such as an action's two inputs). A screen draws them and
//! passes the player's moves in; the state stays the same whatever package shows it.

/// A choice among fixed options. Left and right move through them and wrap at either end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selector<T> {
    options: Vec<T>,
    at: usize,
}

impl<T: PartialEq> Selector<T> {
    /// A selector over `options` with `current` chosen. `None` when there are no options or `current` is not one of
    /// them.
    pub fn new(options: Vec<T>, current: &T) -> Option<Self> {
        let at = options.iter().position(|o| o == current)?;
        Some(Self { options, at })
    }

    /// The option chosen now.
    pub fn value(&self) -> &T {
        &self.options[self.at]
    }

    /// Moves to the next option (`forward`) or the previous one, wrapping. Returns whether the choice changed, which
    /// is false only when there is one option.
    pub fn step(&mut self, forward: bool) -> bool {
        let len = self.options.len();
        let next = match forward {
            true => (self.at + 1) % len,
            false => (self.at + len - 1) % len,
        };
        let changed = next != self.at;
        self.at = next;
        changed
    }

    /// Chooses an option by value. Returns false (and changes nothing) when it is not one of the options.
    pub fn choose(&mut self, value: &T) -> bool {
        let Some(at) = self.options.iter().position(|o| o == value) else { return false };
        self.at = at;
        true
    }
}

/// The two columns of a dual-value row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Primary,
    Secondary,
}

/// A row with a value in each column. An empty column holds `None` (the "---" of the original).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DualValue<T> {
    primary: Option<T>,
    secondary: Option<T>,
}

impl<T> DualValue<T> {
    pub fn new(primary: Option<T>, secondary: Option<T>) -> Self {
        Self { primary, secondary }
    }

    /// The value in a column.
    pub fn get(&self, column: Column) -> Option<&T> {
        match column {
            Column::Primary => self.primary.as_ref(),
            Column::Secondary => self.secondary.as_ref(),
        }
    }

    /// Puts a value in a column, or empties it with `None`. Returns the value that was there.
    pub fn set(&mut self, column: Column, value: Option<T>) -> Option<T> {
        let slot = match column {
            Column::Primary => &mut self.primary,
            Column::Secondary => &mut self.secondary,
        };
        std::mem::replace(slot, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selector_wraps_both_ways() {
        let mut s = Selector::new(vec!["keyboard", "mouse", "pad"], &"keyboard").unwrap();
        assert_eq!(*s.value(), "keyboard");
        assert!(s.step(false));
        assert_eq!(*s.value(), "pad");
        assert!(s.step(true));
        assert_eq!(*s.value(), "keyboard");
    }

    #[test]
    fn a_selector_with_one_option_does_not_change() {
        let mut s = Selector::new(vec![7], &7).unwrap();
        assert!(!s.step(true));
        assert_eq!(*s.value(), 7);
    }

    #[test]
    fn a_selector_needs_its_current_option() {
        assert!(Selector::new(Vec::<u8>::new(), &1).is_none());
        assert!(Selector::new(vec![1, 2], &3).is_none());
    }

    #[test]
    fn choose_moves_to_a_known_option_only() {
        let mut s = Selector::new(vec![1, 2, 3], &1).unwrap();
        assert!(s.choose(&3));
        assert_eq!(*s.value(), 3);
        assert!(!s.choose(&9));
        assert_eq!(*s.value(), 3);
    }

    #[test]
    fn a_dual_value_keeps_each_column_apart() {
        let mut row = DualValue::new(Some('W'), None);
        assert_eq!(row.get(Column::Primary), Some(&'W'));
        assert_eq!(row.get(Column::Secondary), None);
        assert_eq!(row.set(Column::Secondary, Some('↑')), None);
        assert_eq!(row.get(Column::Primary), Some(&'W'));
        assert_eq!(row.set(Column::Primary, None), Some('W'));
        assert_eq!(row.get(Column::Primary), None);
        assert_eq!(row.get(Column::Secondary), Some(&'↑'));
    }
}

//! One entry of a menu list, and the flags that say how the cursor, the pointer and a theme treat it.

/// How the list treats an entry.
///
/// - `highlightable`: the selection can be on it, so a theme draws its selected look and the pointer's hover selects it.
/// - `clickable`: a press on it does something. A press on an entry that is not clickable does nothing at all.
/// - `skip_on_scroll`: the arrows, the d-pad, the stick, the wheel and the page jumps never stop on it.
/// - `centered`: its text is centred across the row instead of sitting in the label column.
/// - `accent_text`: its text is in the colour a highlighted entry's text has.
///
/// Three presets cover the lists: [`EntryFlags::ROW`] (everything that exists today), [`EntryFlags::TITLE`] (a section
/// title: all three behaviours off, centred, accent text) and [`EntryFlags::ACTION`] (a row that does something when it
/// is pressed: all three behaviours on, centred, accent text).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryFlags {
    pub highlightable: bool,
    pub clickable: bool,
    pub skip_on_scroll: bool,
    pub centered: bool,
    pub accent_text: bool,
}

impl EntryFlags {
    /// An ordinary row: it can be highlighted and pressed, the cursor stops on it, its text sits in the label column.
    pub const ROW: Self =
        Self { highlightable: true, clickable: true, skip_on_scroll: false, centered: false, accent_text: false };

    /// A section title: it cannot be highlighted or pressed and the cursor skips it. Centred, in the accent colour.
    pub const TITLE: Self =
        Self { highlightable: false, clickable: false, skip_on_scroll: true, centered: true, accent_text: true };

    /// An action: highlightable and pressable, and the cursor stops on it. Centred, in the accent colour.
    pub const ACTION: Self =
        Self { highlightable: true, clickable: true, skip_on_scroll: false, centered: true, accent_text: true };

    /// Whether scrolling can stop on the entry: it is highlightable and not skipped.
    pub const fn stops_scroll(self) -> bool {
        self.highlightable && !self.skip_on_scroll
    }
}

impl Default for EntryFlags {
    fn default() -> Self {
        Self::ROW
    }
}

/// What an action entry runs when it is pressed.
pub type Callback = Box<dyn FnMut()>;

/// A labelled entry with its flags and, when it does something, its action `A`.
///
/// With every behaviour flag off it is a section title; with them on and an action it is an action row. A host whose rows
/// already carry their own state (a setting, a value) keeps its own row type and only borrows [`EntryFlags`].
pub struct Entry<A> {
    pub label: String,
    pub flags: EntryFlags,
    pub action: Option<A>,
}

impl<A> Entry<A> {
    /// A plain row with no action of its own.
    pub fn row(label: impl Into<String>) -> Self {
        Self { label: label.into(), flags: EntryFlags::ROW, action: None }
    }

    /// A section title.
    pub fn title(label: impl Into<String>) -> Self {
        Self { label: label.into(), flags: EntryFlags::TITLE, action: None }
    }

    /// An action row that runs `action` when it is pressed.
    pub fn action(label: impl Into<String>, action: A) -> Self {
        Self { label: label.into(), flags: EntryFlags::ACTION, action: Some(action) }
    }

    /// The same entry with other flags.
    pub fn with_flags(mut self, flags: EntryFlags) -> Self {
        self.flags = flags;
        self
    }

    /// The action a press on the entry runs: `None` when it is not clickable or has no action.
    pub fn pressed(&self) -> Option<&A> {
        if !self.flags.clickable {
            return None;
        }
        self.action.as_ref()
    }
}

impl Entry<Callback> {
    /// Presses the entry: runs its callback when it is clickable. Returns whether anything ran.
    pub fn press(&mut self) -> bool {
        if !self.flags.clickable {
            return false;
        }
        let Some(action) = self.action.as_mut() else { return false };
        action();
        true
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    #[test]
    fn a_title_is_inert_and_an_action_runs_its_callback() {
        let count = Rc::new(Cell::new(0));
        let seen = Rc::clone(&count);
        let mut action = Entry::action("Open", Box::new(move || seen.set(seen.get() + 1)) as Callback);
        assert!(action.flags.stops_scroll() && action.flags.centered && action.flags.accent_text);
        assert!(action.press() && action.press());
        assert_eq!(count.get(), 2);

        let mut title: Entry<Callback> = Entry::title("Experimental");
        assert!(!title.flags.stops_scroll() && !title.flags.highlightable && !title.flags.clickable);
        assert!(!title.press(), "a title does nothing when pressed");
        assert!(title.pressed().is_none());
    }

    #[test]
    fn an_entry_with_an_action_but_not_clickable_runs_nothing() {
        let count = Rc::new(Cell::new(0));
        let seen = Rc::clone(&count);
        let flags = EntryFlags { clickable: false, ..EntryFlags::ACTION };
        let mut entry = Entry::action("Locked", Box::new(move || seen.set(1)) as Callback).with_flags(flags);
        assert!(!entry.press());
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn the_cursor_stops_only_on_highlightable_entries_that_are_not_skipped() {
        assert!(EntryFlags::ROW.stops_scroll());
        assert!(!EntryFlags::TITLE.stops_scroll());
        let skipped = EntryFlags { skip_on_scroll: true, ..EntryFlags::ROW };
        assert!(!skipped.stops_scroll(), "hover can still select it, the arrows go past");
        let dead = EntryFlags { highlightable: false, ..EntryFlags::ROW };
        assert!(!dead.stops_scroll());
        assert_eq!(EntryFlags::default(), EntryFlags::ROW);
    }
}

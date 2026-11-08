//! Moving a cursor through a list whose entries are not all stops: arrows, d-pad, stick and wheel steps, page jumps, and
//! the first entry a list opens on. An entry is a stop when its flags say scrolling can stop there
//! ([`EntryFlags::stops_scroll`]); a section title is not.

use super::EntryFlags;

/// The first entry the cursor can stop on: where a list opens. `None` when no entry is a stop.
pub fn first_stop(flags: &[EntryFlags]) -> Option<usize> {
    flags.iter().position(|f| f.stops_scroll())
}

/// The last entry the cursor can stop on.
pub fn last_stop(flags: &[EntryFlags]) -> Option<usize> {
    flags.iter().rposition(|f| f.stops_scroll())
}

/// The stop one step after `from` (`forward`) or before it, wrapping round the list and passing every entry that is not
/// a stop. `from` may itself be anything, even past the end. `None` when there is no stop; the entry `from` when it is the
/// only one.
pub fn step(flags: &[EntryFlags], from: usize, forward: bool) -> Option<usize> {
    let n = flags.len();
    if n == 0 {
        return None;
    }
    let from = from.min(n - 1);
    (1..=n)
        .map(|k| match forward {
            true => (from + k) % n,
            false => (from + n - k % n) % n,
        })
        .find(|&i| flags[i].stops_scroll())
}

/// The stop to settle on for a selection at `index`: the entry itself when it is a stop, else the next stop after it,
/// wrapping. For a selection a rebuilt list left on a title.
pub fn settle(flags: &[EntryFlags], index: usize) -> Option<usize> {
    if flags.get(index).is_some_and(|f| f.stops_scroll()) {
        return Some(index);
    }
    step(flags, index, true)
}

/// A page jump: `size` entries forward or back from `from`, without wrapping, landing on the nearest stop in the
/// direction of travel. Past the last stop it lands on the last, before the first on the first. `None` when there is no
/// stop.
pub fn page(flags: &[EntryFlags], from: usize, size: usize, forward: bool) -> Option<usize> {
    let (first, last) = (first_stop(flags)?, last_stop(flags)?);
    let n = flags.len();
    let target = match forward {
        true => from.saturating_add(size.max(1)).min(n - 1),
        false => from.saturating_sub(size.max(1)),
    };
    let found = match forward {
        true => (target..n).find(|&i| flags[i].stops_scroll()),
        false => (0..=target).rev().find(|&i| flags[i].stops_scroll()),
    };
    found.or(Some(match forward {
        true => last,
        false => first,
    }))
}

/// Whether the pointer's hover may select entry `index`: it is highlightable (a skipped entry can still be hovered).
pub fn can_hover(flags: &[EntryFlags], index: usize) -> bool {
    flags.get(index).is_some_and(|f| f.highlightable)
}

/// Whether a press on entry `index` does anything: it is clickable.
pub fn can_press(flags: &[EntryFlags], index: usize) -> bool {
    flags.get(index).is_some_and(|f| f.clickable)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: EntryFlags = EntryFlags::TITLE;
    const R: EntryFlags = EntryFlags::ROW;

    #[test]
    fn a_list_opens_on_the_first_row_after_a_leading_title() {
        assert_eq!(first_stop(&[T, R, R]), Some(1));
        assert_eq!(first_stop(&[R, T, R]), Some(0));
        assert_eq!(last_stop(&[R, R, T]), Some(1));
        assert_eq!(first_stop(&[T, T]), None);
        assert_eq!(first_stop(&[]), None);
    }

    #[test]
    fn steps_skip_titles_and_wrap_over_them() {
        let flags = [R, R, T, R, R, T, R];
        assert_eq!(step(&flags, 1, true), Some(3), "down over the title");
        assert_eq!(step(&flags, 3, false), Some(1), "up over the title");
        assert_eq!(step(&flags, 4, true), Some(6), "down over the second title");
        assert_eq!(step(&flags, 6, true), Some(0), "the end wraps to the top");
        assert_eq!(step(&flags, 0, false), Some(6), "the top wraps to the end");
        let leading = [T, R, R];
        assert_eq!(step(&leading, 2, true), Some(1), "wrapping over a leading title lands on the first row");
        assert_eq!(step(&leading, 1, false), Some(2));
    }

    #[test]
    fn a_lone_stop_stays_and_a_list_without_stops_has_none() {
        assert_eq!(step(&[T, R, T], 1, true), Some(1));
        assert_eq!(step(&[T, R, T], 1, false), Some(1));
        assert_eq!(step(&[T, T], 0, true), None);
        assert_eq!(step(&[], 0, true), None);
    }

    #[test]
    fn an_entry_that_is_skipped_but_highlightable_is_hovered_not_stepped_on() {
        let skipped = EntryFlags { skip_on_scroll: true, ..R };
        let flags = [R, skipped, R];
        assert_eq!(step(&flags, 0, true), Some(2));
        assert!(can_hover(&flags, 1));
        assert!(!can_hover(&[T], 0));
        assert!(can_press(&flags, 1));
        assert!(!can_press(&[T], 0));
        assert!(!can_hover(&flags, 9) && !can_press(&flags, 9));
    }

    #[test]
    fn a_selection_left_on_a_title_settles_on_the_next_row() {
        let flags = [T, R, T, R];
        assert_eq!(settle(&flags, 0), Some(1));
        assert_eq!(settle(&flags, 2), Some(3));
        assert_eq!(settle(&flags, 3), Some(3));
        assert_eq!(settle(&[T, R, T], 2), Some(1), "after the last title it wraps");
    }

    #[test]
    fn page_jumps_land_on_stops_and_stop_at_the_ends() {
        let flags = [T, R, R, R, T, R, R, R, R, R];
        assert_eq!(page(&flags, 1, 3, true), Some(5), "index 4 is the title, so the next row after it");
        assert_eq!(page(&flags, 1, 3, false), Some(1), "back from the top stays on the first stop");
        assert_eq!(page(&flags, 8, 5, true), Some(9), "past the end lands on the last stop");
        assert_eq!(page(&flags, 6, 3, false), Some(3), "back from 6 by 3 is a row");
        assert_eq!(page(&flags, 7, 3, false), Some(3), "back onto the title at 4 goes on to the row before it");
        assert_eq!(page(&flags, 5, 3, false), Some(2));
        assert_eq!(page(&[T, T], 0, 3, true), None);
    }
}

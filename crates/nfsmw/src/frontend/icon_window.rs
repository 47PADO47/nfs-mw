//! The window over an icon strip (`icon_menu.rs`). A package has a fixed number of `OPTION_*` objects, and a menu
//! can list more icons than that. The strip is the two book ends, the icons, and two more book ends; the window is
//! the run of slots that shows part of it. The window stays centred on the selection, so the icons either side of
//! it (three each way reach the edge of the scroller) always have slots. Sliding the window rebinds the slots'
//! textures and shifts the scroll by the same amount, so every icon stays where it was on screen.

/// The strip index of the first slot of a window of `slots` objects over a strip of `strip` entries, centred on
/// `selected` where the strip allows it. A strip that fits the slots is not windowed: the window starts at 0.
pub fn base_for(selected: usize, slots: usize, strip: usize) -> usize {
    if strip <= slots {
        return 0;
    }
    selected.saturating_sub(slots / 2).min(strip - slots)
}

/// How far the scroll moves when the window's first slot goes from `old` to `new`: the slots are `pitch` apart and
/// the strip moves the other way, so the icons keep their places on screen.
pub fn slide_shift(old: usize, new: usize, pitch: f32) -> f32 {
    (new as f32 - old as f32) * pitch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_strip_that_fits_is_not_windowed() {
        assert_eq!(base_for(9, 12, 10), 0);
        assert_eq!(base_for(9, 10, 10), 0);
    }

    #[test]
    fn the_window_follows_the_selection_and_stops_at_the_ends() {
        // Eleven objects over a strip of twelve: the window can only slide by one.
        assert_eq!(base_for(2, 11, 12), 0, "the first icon, with the book ends before it");
        assert_eq!(base_for(5, 11, 12), 0);
        assert_eq!(base_for(6, 11, 12), 1, "past the middle it slides");
        assert_eq!(base_for(11, 11, 12), 1, "and stops at the far book end");
    }

    #[test]
    fn a_slide_keeps_every_icon_where_it_was_on_screen() {
        let pitch = 59.0;
        let (selected, old, new, scroll) = (7_usize, 0_usize, 1_usize, 10.0_f32);
        let before = scroll + (selected - old) as f32 * pitch;
        let after = scroll + slide_shift(old, new, pitch) + (selected - new) as f32 * pitch;
        assert_eq!(before, after);
        assert_eq!(slide_shift(old, new, pitch), 59.0, "the scroll moves right by one pitch");
    }
}

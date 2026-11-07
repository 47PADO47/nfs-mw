//! Digit cells and where each digit stands. Spec: `docs/specs/custom-hud-gauges.md` section 7.
//!
//! A digit atlas is 12 equal cells in one row; cell `n` holds the digit `n`. The gear atlas has the same cells, with cell 10
//! the neutral letter and cell 11 the reverse letter. The units picture has two cells, miles per hour then kilometres.

/// The cells of a digit or gear atlas.
pub const CELLS: u32 = 12;
/// The cell of the neutral letter and of the reverse letter in a gear atlas.
pub const NEUTRAL_CELL: u32 = 10;
pub const REVERSE_CELL: u32 = 11;
/// The speed readout never shows more than this.
pub const MAX_SPEED: u32 = 999;

/// The UV rectangle `[u0, v0, u1, v1]` of cell `cell` of a 12-cell atlas.
pub fn cell_uv(cell: u32) -> [f32; 4] {
    let cells = CELLS as f32;
    [cell as f32 / cells, 0.0, (cell + 1) as f32 / cells, 1.0]
}

/// The width of one cell of a `size`-tall digit, from the picture's size in pixels: the picture's aspect, one cell wide.
pub fn cell_width(size: f32, picture_width: u32, picture_height: u32) -> f32 {
    if picture_height == 0 {
        return 0.0;
    }
    (picture_width as f32 / CELLS as f32) / picture_height as f32 * size
}

/// The atlas cell of a gear given as `-1` reverse, `0` neutral, `1..` forward gears. A gear the atlas has no cell for gives
/// `None` (nothing is drawn).
pub fn gear_cell(gear: i32) -> Option<u32> {
    match gear {
        -1 => Some(REVERSE_CELL),
        0 => Some(NEUTRAL_CELL),
        1..=9 => Some(gear as u32),
        _ => None,
    }
}

/// The UV rectangle of the units picture's half for miles per hour (the left half) or kilometres per hour (the right half).
pub fn units_uv(mph: bool) -> [f32; 4] {
    match mph {
        true => [0.0, 0.0, 0.5, 1.0],
        false => [0.5, 0.0, 1.0, 1.0],
    }
}

/// One place of a readout: where it stands, whether its ghost shows, and the digit it holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    /// The place's number, 0 the rightmost: 0 ones, 1 tens, 2 hundreds, 3 thousands.
    pub place: u32,
    /// Added to `PositionX`: how far the place's right edge stands further from the screen's right edge, reference pixels.
    pub offset: f32,
    /// The ghost "8" behind the digit is drawn.
    pub ghost: bool,
    /// The digit shown, or `None` for a place left empty (a leading zero).
    pub digit: Option<u32>,
}

fn digit_of(value: u32, place: u32) -> u32 {
    (value / 10_u32.pow(place)) % 10
}

fn digit_count(value: u32) -> u32 {
    match value {
        0..=9 => 1,
        10..=99 => 2,
        100..=999 => 3,
        1000..=9999 => 4,
        _ => 5,
    }
}

/// The places of the speed readout (`SPEEDOMETER_DIGITAL`): three, ones digit always shown, no leading zeros. `speed` is cut
/// to 999. Mode 0 keeps all three places, ghosts in all of them; mode 1 has only the places the number needs; mode 2 has the
/// same and centres them, half a step toward the middle for each place the number lacks. Any other mode reads as 0.
pub fn speed_slots(speed: u32, mode: i32, size: f32, distance: f32) -> Vec<Slot> {
    let speed = speed.min(MAX_SPEED);
    let step = size + distance;
    let needed = digit_count(speed);
    let places = match mode {
        1 | 2 => needed,
        _ => 3,
    };
    (0..places)
        .map(|place| {
            let centring = match mode {
                2 => step * 0.5 * (3 - needed) as f32,
                _ => 0.0,
            };
            let shown = place == 0 || speed / 10_u32.pow(place) != 0;
            Slot {
                place,
                offset: place as f32 * step + centring,
                ghost: true,
                digit: shown.then(|| digit_of(speed, place)),
            }
        })
        .collect()
}

/// The places of the rev counter (`TACHOMETER_DIGITAL`), from the rpm as a whole number: mode 1 has only the places the
/// number needs (none above 9999), any other mode always four, leading zeros shown.
pub fn rev_slots(rpm: u32, mode: i32, size: f32, distance: f32) -> Vec<Slot> {
    let step = size + distance;
    let places = match mode {
        1 => digit_count(rpm).min(5),
        _ => 4,
    };
    if places > 4 {
        return Vec::new();
    }
    (0..places)
        .map(|place| Slot { place, offset: place as f32 * step, ghost: true, digit: Some(digit_of(rpm, place)) })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digits(slots: &[Slot]) -> Vec<Option<u32>> {
        slots.iter().map(|slot| slot.digit).collect()
    }

    #[test]
    fn cells_cut_the_atlas_in_twelve() {
        assert_eq!(cell_uv(0), [0.0, 0.0, 1.0 / 12.0, 1.0]);
        assert_eq!(cell_uv(11)[2], 1.0);
        assert!((cell_uv(5)[0] - 5.0 / 12.0).abs() < 1e-6);
        // A 1368 x 114 atlas has 114 x 114 cells: a 65-tall digit is 65 wide.
        assert!((cell_width(65.0, 1368, 114) - 65.0).abs() < 1e-3);
        assert_eq!(cell_width(65.0, 1368, 0), 0.0);
    }

    #[test]
    fn gears_pick_the_neutral_and_reverse_cells() {
        assert_eq!(gear_cell(-1), Some(11));
        assert_eq!(gear_cell(0), Some(10));
        assert_eq!(gear_cell(1), Some(1));
        assert_eq!(gear_cell(6), Some(6));
        assert_eq!(gear_cell(10), None);
        assert_eq!(gear_cell(-2), None);
    }

    #[test]
    fn the_units_picture_has_miles_on_the_left() {
        assert_eq!(units_uv(true), [0.0, 0.0, 0.5, 1.0]);
        assert_eq!(units_uv(false), [0.5, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn mode_0_keeps_three_places_and_blanks_leading_zeros() {
        let slots = speed_slots(42, 0, 65.0, -35.0);
        assert_eq!(slots.len(), 3);
        assert_eq!(digits(&slots), vec![Some(2), Some(4), None], "ones, tens, then an empty hundreds");
        assert_eq!(slots.iter().map(|s| s.offset).collect::<Vec<_>>(), vec![0.0, 30.0, 60.0]);
        assert!(slots.iter().all(|slot| slot.ghost), "mode 0 draws every ghost");
    }

    #[test]
    fn the_ones_digit_always_shows_and_speeds_stop_at_999() {
        assert_eq!(digits(&speed_slots(0, 0, 10.0, 0.0)), vec![Some(0), None, None]);
        assert_eq!(digits(&speed_slots(1234, 0, 10.0, 0.0)), vec![Some(9), Some(9), Some(9)]);
        assert_eq!(digits(&speed_slots(305, 1, 10.0, 0.0)), vec![Some(5), Some(0), Some(3)], "a zero inside shows");
    }

    #[test]
    fn mode_1_has_only_the_needed_places_at_the_same_positions() {
        let slots = speed_slots(7, 1, 65.0, -35.0);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].offset, 0.0);
        assert_eq!(speed_slots(130, 1, 65.0, -35.0).len(), 3);
    }

    #[test]
    fn mode_2_centres_the_digits_half_a_step_per_missing_place() {
        let step = 30.0;
        let one = speed_slots(7, 2, 65.0, -35.0);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].offset, step, "a lone digit stands where the middle of three would");
        let two = speed_slots(42, 2, 65.0, -35.0);
        assert_eq!(two.iter().map(|s| s.offset).collect::<Vec<_>>(), vec![15.0, 45.0]);
        let three = speed_slots(130, 2, 65.0, -35.0);
        assert_eq!(three.iter().map(|s| s.offset).collect::<Vec<_>>(), vec![0.0, 30.0, 60.0]);
    }

    #[test]
    fn the_rev_counter_shows_four_places_with_leading_zeros() {
        let slots = rev_slots(850, 0, 40.0, -10.0);
        assert_eq!(digits(&slots), vec![Some(0), Some(5), Some(8), Some(0)]);
        assert_eq!(slots[3].offset, 90.0);
        assert_eq!(digits(&rev_slots(7250, 0, 40.0, 0.0)), vec![Some(0), Some(5), Some(2), Some(7)]);
    }

    #[test]
    fn the_rev_counter_in_mode_1_needs_only_its_digits_and_stops_above_9999() {
        assert_eq!(rev_slots(850, 1, 40.0, 0.0).len(), 3);
        assert_eq!(rev_slots(9, 1, 40.0, 0.0).len(), 1);
        assert_eq!(rev_slots(9999, 1, 40.0, 0.0).len(), 4);
        assert!(rev_slots(10_000, 1, 40.0, 0.0).is_empty());
    }
}

//! The elements that are not dials: digit rows, the gear, the units picture, bars, the shift light and the background.
//! Spec: `docs/specs/custom-hud-gauges.md` sections 7 and 9.

use crate::digits::{self, Slot};
use crate::draw::{Pictures, Quad};
use crate::element::{Bar, Digital, Icon, Picture};
use crate::place::{Placement, Rect};
use crate::value::Colour;

const FULL: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
/// The cell of the ghost "8" behind a digit.
const GHOST_CELL: u32 = 8;

fn quad(picture: &Picture, file: &str, rect: Rect, uv: [f32; 4], colour: Colour) -> Quad {
    Quad { picture: file.to_owned(), additive: picture.additive, rect, uv, colour, turn: None, sector: None }
}

/// A row of digits at the places `slots` gives: each place's ghost, then its digit. Nothing when the picture did not load.
pub(crate) fn digit_row(
    row: &Digital,
    slots: &[Slot],
    pictures: &dyn Pictures,
    placement: &Placement,
    out: &mut Vec<Quad>,
) {
    let common = &row.common;
    for slot in slots {
        let file = match slot.place {
            1 if !row.tens_file.is_empty() => &row.tens_file,
            2 if !row.hundreds_file.is_empty() => &row.hundreds_file,
            _ => &row.picture.file,
        };
        let Some((width, height)) = pictures.size(file) else { continue };
        let cell = digits::cell_width(common.size, width, height);
        let rect = placement.rect(cell, common.size, common.x + slot.offset, common.y);
        if slot.ghost && !row.ghost.is_none() {
            out.push(quad(&row.picture, file, rect, digits::cell_uv(GHOST_CELL), row.ghost));
        }
        if let Some(digit) = slot.digit {
            out.push(quad(&row.picture, file, rect, digits::cell_uv(digit), common.colour));
        }
    }
}

/// The speed readout for a speed in the shown unit.
pub(crate) fn speed(row: &Digital, speed: f32, pictures: &dyn Pictures, placement: &Placement, out: &mut Vec<Quad>) {
    let whole = speed.max(0.0).floor().min(digits::MAX_SPEED as f32) as u32;
    let slots = digits::speed_slots(whole, row.mode, row.common.size, row.distance);
    digit_row(row, &slots, pictures, placement, out);
}

/// The rev counter for an engine speed in rpm.
pub(crate) fn revs(row: &Digital, rpm: f32, pictures: &dyn Pictures, placement: &Placement, out: &mut Vec<Quad>) {
    let whole = rpm.max(0.0).floor().min(99_999.0) as u32;
    let slots = digits::rev_slots(whole, row.mode, row.common.size, row.distance);
    digit_row(row, &slots, pictures, placement, out);
}

/// The gear letter or digit: one cell, with its ghost.
pub(crate) fn gear(row: &Digital, gear: i32, pictures: &dyn Pictures, placement: &Placement, out: &mut Vec<Quad>) {
    let slot = Slot { place: 0, offset: 0.0, ghost: true, digit: digits::gear_cell(gear) };
    digit_row(row, &[slot], pictures, placement, out);
}

/// The units picture: miles per hour on the left half, kilometres per hour on the right, each `2 * Size` by `Size`.
pub(crate) fn units(icon: &Icon, mph: bool, placement: &Placement, out: &mut Vec<Quad>) {
    if !icon.picture.is_set() {
        return;
    }
    let common = &icon.common;
    let rect = placement.rect(2.0 * common.size, common.size, common.x, common.y);
    out.push(quad(&icon.picture, &icon.picture.file, rect, digits::units_uv(mph), common.colour));
}

/// A picture that fills the element's square (the background, the shift light).
pub(crate) fn square(icon: &Icon, colour: Colour, placement: &Placement, out: &mut Vec<Quad>) {
    if !icon.picture.is_set() {
        return;
    }
    let common = &icon.common;
    let rect = placement.rect(common.size, common.size, common.x, common.y);
    out.push(quad(&icon.picture, &icon.picture.file, rect, FULL, colour));
}

/// The part of a bar's square that is filled, as fractions `[x0, y0, x1, y1]` of the square, which are also the picture's
/// UV rectangle. `fraction` is clamped to 0..1.
pub fn bar_fill(direction: i32, fraction: f32) -> [f32; 4] {
    let f = fraction.clamp(0.0, 1.0);
    match direction {
        1 => [0.0, 1.0 - f, 1.0, 1.0],
        2 => [1.0 - f, 0.0, 1.0, 1.0],
        _ => [0.0, 0.0, f, 1.0],
    }
}

/// A bar: its background, then the filled part for `value`.
pub(crate) fn bar(row: &Bar, value: f32, placement: &Placement, out: &mut Vec<Quad>) {
    let common = &row.common;
    let square = placement.rect(common.size, common.size, common.x, common.y);
    if row.background.is_set() {
        out.push(quad(&row.background, &row.background.file, square, FULL, row.background.colour));
    }
    if !row.filled.is_set() || row.value <= 0.0 {
        return;
    }
    let [x0, y0, x1, y1] = bar_fill(row.direction, value / row.value);
    let (left, top) = (square.x - square.w / 2.0, square.y - square.h / 2.0);
    let part = Rect {
        x: left + (x0 + x1) / 2.0 * square.w,
        y: top + (y0 + y1) / 2.0 * square.h,
        w: (x1 - x0) * square.w,
        h: (y1 - y0) * square.h,
    };
    out.push(quad(&row.filled, &row.filled.file, part, [x0, y0, x1, y1], row.filled.colour));
}

/// Whether the shift light is on: the engine is within half a thousand rpm of the red line.
pub fn shift_light_on(rpm: f32, red_line: f32) -> bool {
    rpm / 1000.0 + 0.5 >= red_line / 1000.0
}

/// How the shift light shows this frame: `None` hidden, `Some(None)` in its own colour, `Some(Some(alpha))` with its alpha
/// byte replaced (the pulse). `timer` is the blink timer, kept from frame to frame; it resets when the light is off.
pub fn blink(mode: i32, on: bool, dt: f32, timer: &mut f32) -> Option<Option<u8>> {
    if !on {
        *timer = 0.0;
        return None;
    }
    match mode {
        1 => {
            *timer += dt;
            if *timer > 1.0 {
                *timer = 0.0;
            }
            (*timer > 0.5).then_some(None)
        }
        2 => {
            *timer += dt;
            if *timer > 0.5 {
                *timer = 0.0;
            }
            let t = match *timer > 0.25 {
                true => 0.5 - *timer,
                false => *timer,
            };
            Some(Some((t * 2.0 * 256.0).clamp(0.0, 255.0) as u8))
        }
        _ => Some(None),
    }
}

/// The shift light.
pub(crate) fn shift_light(icon: &Icon, show: Option<Option<u8>>, placement: &Placement, out: &mut Vec<Quad>) {
    let Some(alpha) = show else { return };
    let colour = match alpha {
        Some(alpha) => icon.common.colour.with_alpha(alpha),
        None => icon.common.colour,
    };
    square(icon, colour, placement, out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::Skin;

    struct Sizes;
    impl Pictures for Sizes {
        fn size(&self, file: &str) -> Option<(u32, u32)> {
            match file {
                "digits.dds" | "gear.dds" | "tens.dds" => Some((1200, 100)),
                "units.dds" => Some((512, 128)),
                _ => None,
            }
        }
    }

    fn placement() -> Placement {
        Placement::new(480.0 * 16.0 / 9.0, 480.0, 1.0, 0.0, 0.0)
    }

    fn row(extra: &str) -> Digital {
        let text = format!(
            "[SPEEDOMETER_DIGITAL]\nEnabled = 1\nSize = 65\nPositionX = 130\nPositionY = 83\nColor = 0xFFFFFFFF\nDistance = -35\nDigitsTexture = digits.dds\n{extra}"
        );
        Skin::parse(&text).speed_digits.expect("enabled")
    }

    #[test]
    fn the_speed_readout_draws_the_digits_of_a_whole_speed_from_right_to_left() {
        let mut out = Vec::new();
        speed(&row("DigitsMode = 0\n"), 42.9, &Sizes, &placement(), &mut out);
        assert_eq!(out.len(), 2, "no ghost colour, and the hundreds place is empty");
        let cells: Vec<f32> = out.iter().map(|q| (q.uv[0] * 12.0).round()).collect();
        assert_eq!(cells, vec![2.0, 4.0], "the ones digit (2) first, then the tens (4)");
        assert!(out[1].rect.x < out[0].rect.x, "the tens digit stands to the left of the ones");
        // Each cell is the picture's aspect: 100 wide for a 100-tall picture, scaled to Size 65.
        let unit = placement().unit();
        assert!((out[0].rect.w - 65.0 * unit).abs() < 1e-3);
        assert!((out[0].rect.h - 65.0 * unit).abs() < 1e-3);
    }

    #[test]
    fn a_ghost_colour_draws_an_eight_behind_each_place() {
        let mut out = Vec::new();
        speed(&row("DigitsMode = 1\nBackgroundColor = 0x22FFFFFF\n"), 7.0, &Sizes, &placement(), &mut out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].colour, Colour(0x22FF_FFFF));
        assert_eq!((out[0].uv[0] * 12.0).round(), 8.0, "the ghost is the digit 8");
        assert_eq!((out[1].uv[0] * 12.0).round(), 7.0);
    }

    #[test]
    fn the_tens_picture_replaces_the_digit_atlas_for_that_place_only() {
        let mut out = Vec::new();
        speed(&row("DigitsTexture1 = tens.dds\n"), 55.0, &Sizes, &placement(), &mut out);
        let files: Vec<&str> = out.iter().map(|q| q.picture.as_str()).collect();
        assert_eq!(files, vec!["digits.dds", "tens.dds"]);
    }

    #[test]
    fn a_readout_whose_picture_did_not_load_draws_nothing() {
        let mut out = Vec::new();
        let missing = row("").picture.file.replace("digits", "missing");
        let mut other = row("");
        other.picture.file = missing;
        speed(&other, 55.0, &Sizes, &placement(), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn the_rev_counter_draws_four_places() {
        let mut out = Vec::new();
        revs(&row(""), 850.9, &Sizes, &placement(), &mut out);
        assert_eq!(out.len(), 4);
        let cells: Vec<f32> = out.iter().map(|q| (q.uv[0] * 12.0).round()).collect();
        assert_eq!(cells, vec![0.0, 5.0, 8.0, 0.0]);
    }

    #[test]
    fn the_gear_shows_the_letter_cells() {
        let mut out = Vec::new();
        let g = Skin::parse("[GEAR]\nEnabled = 1\nSize = 130\nDigitsTexture = gear.dds\nColor = 0xFFFFFFFF\n")
            .gear
            .expect("on");
        gear(&g, 0, &Sizes, &placement(), &mut out);
        gear(&g, -1, &Sizes, &placement(), &mut out);
        gear(&g, 3, &Sizes, &placement(), &mut out);
        gear(&g, 12, &Sizes, &placement(), &mut out);
        let cells: Vec<f32> = out.iter().map(|q| (q.uv[0] * 12.0).round()).collect();
        assert_eq!(cells, vec![10.0, 11.0, 3.0], "a gear the atlas lacks draws nothing");
    }

    #[test]
    fn the_units_picture_shows_the_half_for_the_unit() {
        let icon =
            Skin::parse("[UNITS]\nEnabled = 1\nSize = 64\nPositionX = 128\nPositionY = 45\nUnitsTexture = units.dds\n")
                .units
                .expect("on");
        let (mut miles, mut km) = (Vec::new(), Vec::new());
        units(&icon, true, &placement(), &mut miles);
        units(&icon, false, &placement(), &mut km);
        assert_eq!(miles[0].uv, [0.0, 0.0, 0.5, 1.0]);
        assert_eq!(km[0].uv, [0.5, 0.0, 1.0, 1.0]);
        assert!((miles[0].rect.w / miles[0].rect.h - 2.0).abs() < 1e-4, "each half is twice as wide as it is tall");
    }

    #[test]
    fn bars_fill_from_the_side_their_direction_names() {
        assert_eq!(bar_fill(0, 0.25), [0.0, 0.0, 0.25, 1.0]);
        assert_eq!(bar_fill(1, 0.25), [0.0, 0.75, 1.0, 1.0]);
        assert_eq!(bar_fill(2, 0.25), [0.75, 0.0, 1.0, 1.0]);
        assert_eq!(bar_fill(0, 3.0), [0.0, 0.0, 1.0, 1.0]);
        assert_eq!(bar_fill(2, -1.0), [1.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn a_bar_draws_the_filled_part_in_its_own_corner_of_the_square() {
        let text = "[NOS_LINIAR]\nEnabled = 1\nSize = 100\nPositionX = 10\nPositionY = 10\nValue = 1\nDirection = 2\nFilledTexture = fill.dds\nFilledColor = 0xFFFFFFFF\n";
        let row = Skin::parse(text).nitrous_bar.expect("on");
        let mut out = Vec::new();
        bar(&row, 0.5, &placement(), &mut out);
        assert_eq!(out.len(), 1, "no background picture");
        let square = placement().rect(100.0, 100.0, 10.0, 10.0);
        let part = out[0].rect;
        assert!((part.w - square.w / 2.0).abs() < 1e-4);
        assert!(
            (part.x + part.w / 2.0 - (square.x + square.w / 2.0)).abs() < 1e-4,
            "right-aligned: it fills from the right"
        );
        assert_eq!(out[0].uv, [0.5, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn the_shift_light_comes_on_within_half_a_thousand_rpm_of_the_red_line() {
        assert!(shift_light_on(7100.0, 7500.0));
        assert!(!shift_light_on(6900.0, 7500.0));
        assert!(shift_light_on(7600.0, 7500.0));
    }

    #[test]
    fn the_blink_modes_follow_their_timers() {
        let mut timer = 0.0;
        assert_eq!(blink(0, true, 0.1, &mut timer), Some(None), "steady");
        assert_eq!(blink(1, true, 0.3, &mut timer), None, "mode 1 is dark in the first half second");
        assert_eq!(blink(1, true, 0.3, &mut timer), Some(None), "and lit after it");
        assert_eq!(blink(1, true, 0.6, &mut timer), None, "the timer wrapped past one second");
        assert_eq!(blink(1, false, 0.1, &mut timer), None);
        assert_eq!(timer, 0.0, "the light going out resets the timer");
        let mut pulse = 0.0;
        let peak = blink(2, true, 0.25, &mut pulse).expect("shown").expect("an alpha");
        assert_eq!(peak, 128);
        let down = blink(2, true, 0.1, &mut pulse).expect("shown").expect("an alpha");
        assert_eq!(down, ((0.5_f32 - 0.35) * 2.0 * 256.0) as u8);
    }

    #[test]
    fn a_pulse_replaces_the_alpha_of_the_icon_colour() {
        let icon = Skin::parse("[SHIFT_ICON]\nEnabled = 1\nSize = 50\nColor = 0xFFFF8000\nIconTexture = shift.dds\n")
            .shift_light
            .expect("on");
        let mut out = Vec::new();
        shift_light(&icon, Some(Some(0x40)), &placement(), &mut out);
        shift_light(&icon, None, &placement(), &mut out);
        shift_light(&icon, Some(None), &placement(), &mut out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].colour, Colour(0x40FF_8000));
        assert_eq!(out[1].colour, Colour(0xFFFF_8000));
    }
}

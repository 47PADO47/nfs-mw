//! The new-career setup (`docs/specs/career-flow.md` section 3, `docs/formats/career-save.md` sections 3.2 and 3.3).
//!
//! [`new_career_with`] starts from [`CareerSettings::new_career`] and adds the parts the spec gives on top of it: the
//! Underground 2 cash bonus and the bonus-content flag. The profile serial is built by [`generate_serial`] from the
//! game's global generator ([`SerialRng`], `career-flow.md` 3.1). The four letters of the serial come from the caller:
//! their source is open in the spec.

use super::{CareerSettings, Flags, UG2_BONUS_CASH};

/// Range of the serial number draw: `draw(0x19B3)`, that is 6579 values (`career-flow.md` 3.1).
pub const SERIAL_NUMBER_RANGE: u32 = 0x19B3;
/// Added to the serial number draw, so the number is `1069..=7647` (`0x42D` = 1069).
pub const SERIAL_NUMBER_OFFSET: u32 = 0x42D;
/// Longest serial text in characters. The original writes 12 characters and a NUL.
pub const SERIAL_TEXT_MAX: usize = 12;
/// Only the first 4 letters are used (`%.4s`).
const SERIAL_LETTERS_MAX: usize = 4;

/// Options for a new career. The defaults give the plain new career of [`CareerSettings::new_career`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewCareer {
    /// True when an `NFS Underground 2` profile folder exists (`career-flow.md` section 3, step 7). The folder check
    /// belongs to the caller. Adds [`UG2_BONUS_CASH`] to the cash.
    pub ug2_bonus: bool,
    /// The result of the bonus-content probe (`career-flow.md` section 3, step 4). Sets flag bit `0x20000`
    /// ([`Flags::BONUS_POPUP_QUEUED`]). Unconfirmed: the probe itself is not known, and the popup queue entry it
    /// should also add (type 2) is not written here because its record layout is not in the docs.
    pub bonus_content: bool,
    /// The profile serial text, 16 bytes zero padded (see [`generate_serial`] and [`format_serial`]). `None` leaves it
    /// empty, as [`CareerSettings::new_career`] does.
    pub serial: Option<[u8; 16]>,
}

/// The settings of a new career built from `opts`: the starter car `M3GTRCAREERSTART`, stage 16, flags with only
/// "career started" plus the options, cash 0 plus the optional bonus. The graph node `16.1.0` is not part of the
/// settings; the caller activates [`super::PROLOGUE_START_NODE`].
pub fn new_career_with(opts: &NewCareer) -> CareerSettings {
    let mut settings = CareerSettings::new_career();
    if opts.ug2_bonus {
        settings.add_cash(UG2_BONUS_CASH);
    }
    if opts.bonus_content {
        settings.flags.insert(Flags::BONUS_POPUP_QUEUED);
    }
    if let Some(serial) = opts.serial {
        settings.serial = serial;
    }
    settings
}

/// The game's global random generator (`docs/specs/career-flow.md` section 3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialRng {
    state: u32,
}

impl SerialRng {
    /// The static initial state `0x12345678` (`career-flow.md` 3.1 **[exe]**).
    pub const START_STATE: u32 = 0x1234_5678;

    /// A generator at `state`, for tests or for replaying earlier draws.
    pub fn with_state(state: u32) -> SerialRng {
        SerialRng { state }
    }

    /// The current state.
    pub fn state(&self) -> u32 {
        self.state
    }

    /// One draw with range `n`. Returns `state mod n` from the old state, then steps the state:
    /// `u = s ^ 0x1D872B41; t = u ^ (u >> 5); s = (t << 27) ^ t ^ u`.
    ///
    /// `n == 0` returns 0 and leaves the state alone. The spec says only "n = 0 returns 0"; whether the state still
    /// steps in that case is unconfirmed.
    pub fn draw(&mut self, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let r = self.state % n;
        let u = self.state ^ 0x1D87_2B41;
        let t = u ^ (u >> 5);
        self.state = (t << 27) ^ t ^ u;
        r
    }
}

impl Default for SerialRng {
    /// The start state, not zero: `SerialRng::default().state() == 0x12345678`.
    fn default() -> Self {
        SerialRng::with_state(SerialRng::START_STATE)
    }
}

/// Generates a profile serial (`career-flow.md` 3.1). One draw with range [`SERIAL_NUMBER_RANGE`] plus
/// [`SERIAL_NUMBER_OFFSET`] gives the number, `1069..=7647`. `letters` are the source of the `%.4s` argument. The spec
/// does not say where the original gets them (open item), so the caller passes them in. Formatting is in
/// [`format_serial`].
pub fn generate_serial(rng: &mut SerialRng, letters: &[u8]) -> [u8; 16] {
    let number = rng.draw(SERIAL_NUMBER_RANGE) + SERIAL_NUMBER_OFFSET;
    format_serial(number, letters)
}

/// Formats the serial text `MW-<number>-<letters>XXX` the way the original does (`career-flow.md` 3.1, format string
/// `"MW-%d-%.4sXXX"`).
///
/// - `letters` are read up to the first NUL and cut to 4 bytes (`%.4s`). Lowercase `a`..`z` become uppercase.
/// - The text is cut to [`SERIAL_TEXT_MAX`] = 12 characters, because the original's buffer is 13 bytes with a NUL at
///   index 12. So `XXX` only shows when the letters are short: 4 letters with a 4-digit number give `MW-NNNN-LLLL`,
///   2 letters give `MW-NNNN-LLXX`, 0 letters give `MW-NNNN-XXX`.
/// - The result is 16 bytes: the text, a NUL at index 12, then zeros. The original writes 13 bytes. Bytes 13..15 are
///   zero here, but the original may leave them as they were. This is unconfirmed (the save field is 16 bytes).
pub fn format_serial(number: u32, letters: &[u8]) -> [u8; 16] {
    let letters = letters.iter().take_while(|&&b| b != 0).take(SERIAL_LETTERS_MAX);
    let mut text = format!("MW-{number}-").into_bytes();
    text.extend(letters.map(|b| b.to_ascii_uppercase()));
    text.extend_from_slice(b"XXX");
    let mut serial = [0u8; 16];
    let n = text.len().min(SERIAL_TEXT_MAX);
    serial[..n].copy_from_slice(&text[..n]);
    serial
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{PROLOGUE_STAGE, STARTER_CAR_HANDLE};

    #[test]
    fn the_default_options_give_the_plain_new_career() {
        let settings = new_career_with(&NewCareer::default());
        assert_eq!(settings, CareerSettings::new_career());
        assert_eq!(settings.car_handle, STARTER_CAR_HANDLE);
        assert_eq!(settings.stage, PROLOGUE_STAGE);
        assert_eq!(settings.cash, 0);
        assert_eq!(settings.flags, Flags::CAREER_STARTED);
        assert_eq!(settings.serial, [0u8; 16]);
    }

    #[test]
    fn the_ug2_bonus_adds_ten_thousand_cash_only_when_set() {
        let with = new_career_with(&NewCareer { ug2_bonus: true, ..NewCareer::default() });
        assert_eq!(with.cash, 10_000);
        let without = new_career_with(&NewCareer { ug2_bonus: false, ..NewCareer::default() });
        assert_eq!(without.cash, 0);
    }

    #[test]
    fn the_bonus_content_probe_sets_only_its_flag_bit() {
        let settings = new_career_with(&NewCareer { bonus_content: true, ..NewCareer::default() });
        assert_eq!(settings.flags, Flags::CAREER_STARTED.with(Flags::BONUS_POPUP_QUEUED));
        assert_eq!(settings.flags.0, 0x20001);
    }

    #[test]
    fn a_given_serial_is_stored_as_is() {
        let serial = format_serial(7, b"ABCD");
        let settings = new_career_with(&NewCareer { serial: Some(serial), ..NewCareer::default() });
        assert_eq!(settings.serial, serial);
        assert_eq!(settings.serial_text(), "MW-7-ABCDXXX");
    }

    #[test]
    fn the_start_state_is_0x12345678() {
        assert_eq!(SerialRng::default().state(), 0x1234_5678);
        assert_eq!(SerialRng::START_STATE, 0x1234_5678);
    }

    #[test]
    fn the_first_draws_from_the_start_state_follow_the_spec_step() {
        // s0 = 0x12345678 = 305_419_896.
        // Draw 1, n = 6579:
        //   r  = s0 mod 6579. 6579 * 46423 = 305_416_917, and 305_419_896 - 305_416_917 = 2979, so r = 2979.
        //   u  = 0x12345678 ^ 0x1D872B41 = 0x0FB37D39
        //   t  = u ^ (u >> 5) = 0x0FCEE6D0
        //   s1 = (t << 27) ^ t ^ u. Only the low 5 bits of t reach bit 31: 0xD0 & 0x1F = 0x10, and 0x10 << 27 = 0x80000000.
        //        t ^ u = u >> 5. So s1 = 0x80000000 ^ (u >> 5) = 0x807D9BE9.
        // Draws 2..4 continue the same step from the state each time (computed the same way).
        let mut rng = SerialRng::default();
        assert_eq!(rng.draw(6579), 2979);
        assert_eq!(rng.state(), 0x807D_9BE9);
        assert_eq!(rng.draw(6579), 947);
        assert_eq!(rng.draw(6579), 4314);
        assert_eq!(rng.draw(6579), 3612);
    }

    #[test]
    fn a_draw_with_range_zero_returns_zero_and_keeps_the_state() {
        let mut rng = SerialRng::default();
        assert_eq!(rng.draw(0), 0);
        assert_eq!(rng.state(), SerialRng::START_STATE);
    }

    #[test]
    fn the_number_is_drawn_from_the_range_1069_to_7647() {
        let mut rng = SerialRng::default();
        for _ in 0..10_000 {
            let serial = generate_serial(&mut rng, b"ABCD");
            let text = &serial[3..7];
            let number: u32 = std::str::from_utf8(text).unwrap().parse().unwrap();
            assert!((1069..=7647).contains(&number), "number {number} out of range");
        }
    }

    #[test]
    fn the_first_generated_number_is_2979_plus_1069() {
        // draw(6579) = 2979 (see the draw test), so the number is 4048.
        let mut rng = SerialRng::default();
        assert_eq!(generate_serial(&mut rng, b"ABCD")[..12], *b"MW-4048-ABCD");
    }

    #[test]
    fn four_letters_fill_the_text_and_cut_the_tail() {
        let serial = format_serial(4048, b"ABCD");
        assert_eq!(serial[..12], *b"MW-4048-ABCD");
        assert_eq!(serial[12..], [0u8; 4]);
    }

    #[test]
    fn two_letters_leave_the_first_xs_of_the_tail() {
        assert_eq!(format_serial(4048, b"AB")[..12], *b"MW-4048-ABXX");
    }

    #[test]
    fn no_letters_give_the_tail_xxx() {
        assert_eq!(format_serial(4048, b"")[..12], *b"MW-4048-XXX ");
    }

    #[test]
    fn lowercase_letters_are_uppercased() {
        assert_eq!(format_serial(4048, b"abcd")[..12], *b"MW-4048-ABCD");
        assert_eq!(format_serial(4048, b"aB1z")[..12], *b"MW-4048-AB1Z");
    }

    #[test]
    fn only_the_first_four_letters_are_used() {
        assert_eq!(format_serial(3, b"ABCDEFG")[..12], *b"MW-3-ABCDXXX");
    }

    #[test]
    fn a_nul_in_the_letters_ends_them() {
        assert_eq!(format_serial(4048, b"AB\0CD")[..12], *b"MW-4048-ABXX");
    }

    #[test]
    fn a_short_number_shows_the_full_xxx_tail() {
        let serial = format_serial(7, b"ABCD");
        assert_eq!(serial[..12], *b"MW-7-ABCDXXX");
    }

    #[test]
    fn a_long_number_is_cut_to_twelve_characters() {
        let serial = format_serial(123_456_789, b"ABCD");
        assert_eq!(serial[..12], *b"MW-123456789");
        assert_eq!(serial[12..], [0u8; 4]);
    }
}

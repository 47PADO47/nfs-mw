//! The file write as data and the rule for what is reported
//! (`docs/specs/career-save-screens.md` section 6).
//!
//! The original writes `<root>/<alias>/<alias>` in one open / write / close: the file is opened with truncate and
//! written in place (no temporary file, no rename, no flush), header then descriptor then profile. Only a failure to
//! open the file is an error; a short write or a full disk is ignored because the status is reset to success after
//! the close. This module gives the chunks and the outcome rule; it does no I/O.

use crate::save::ProfileSave;
use crate::save::layout::{DESCRIPTOR_LEN, HEADER_LEN, PROFILE_AT};

/// The library error code of a failed open (also folder creation, invalid characters, a read-only file).
pub const OPEN_FAILED_CODE: u8 = 7;

/// One chunk of the write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Which part of the file it is.
    pub part: Part,
    /// Its bytes.
    pub bytes: Vec<u8>,
}

/// The three parts, in the order they are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The 0x1C-byte header (magic, total size, sizes, the checksums).
    Header,
    /// The 8-byte descriptor (`0x10D` and the profile size).
    Descriptor,
    /// The 0xF848-byte profile buffer.
    Profile,
}

/// The ordered chunks of a save: header (0x1C), descriptor (8), profile (0xF848). Concatenated they are the file
/// that [`ProfileSave::to_bytes`] returns: the profile MD5 and the three checksums are computed here the way the
/// library prepares them before the write.
pub fn write_plan(save: &ProfileSave) -> [Chunk; 3] {
    let file = save.to_bytes();
    let (header, rest) = file.split_at(HEADER_LEN);
    let (descriptor, profile) = rest.split_at(DESCRIPTOR_LEN);
    debug_assert_eq!(HEADER_LEN + DESCRIPTOR_LEN, PROFILE_AT);
    [
        Chunk { part: Part::Header, bytes: header.to_vec() },
        Chunk { part: Part::Descriptor, bytes: descriptor.to_vec() },
        Chunk { part: Part::Profile, bytes: profile.to_vec() },
    ]
}

/// What the steps of the write found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteSteps {
    /// The alias folder already exists.
    pub dir_exists: bool,
    /// The library asks prompts (the manual save). A background autosave auto-answers the overwrite question.
    pub interactive: bool,
    /// The overwrite question was answered Yes (ignored when no question is asked).
    pub overwrite_confirmed: bool,
    /// `mkdir` is attempted and its result ignored; this is whether the file opened for writing (write only,
    /// create, truncate, binary).
    pub open_ok: bool,
    /// Any of the three writes failed (short write, full disk). Ignored.
    pub write_error: bool,
}

/// The result of the write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    /// Success, including every case where a write failed after the open.
    Ok,
    /// The overwrite question was answered No: status "cancelled", nothing written.
    Cancelled,
    /// The file or folder could not be opened (code 7): "unable to save X".
    OpenFailed,
}

impl WriteOutcome {
    /// The rule: an existing alias in an interactive save asks first and No cancels; then only an open failure is an
    /// error. Write errors are not reported.
    pub fn resolve(steps: &WriteSteps) -> WriteOutcome {
        if steps.dir_exists && steps.interactive && !steps.overwrite_confirmed {
            return WriteOutcome::Cancelled;
        }
        if !steps.open_ok {
            return WriteOutcome::OpenFailed;
        }
        WriteOutcome::Ok
    }

    /// The library error code, if any.
    pub const fn code(self) -> Option<u8> {
        match self {
            WriteOutcome::OpenFailed => Some(OPEN_FAILED_CODE),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::save::layout::{FILE_LEN, PROFILE_LEN};

    fn steps() -> WriteSteps {
        WriteSteps {
            dir_exists: false,
            interactive: true,
            overwrite_confirmed: false,
            open_ok: true,
            write_error: false,
        }
    }

    fn save() -> ProfileSave {
        ProfileSave::from_profile_buffer(&vec![0x5Au8; PROFILE_LEN]).unwrap()
    }

    #[test]
    fn the_plan_is_header_descriptor_profile_in_that_order_with_the_documented_sizes() {
        let plan = write_plan(&save());
        let parts: Vec<Part> = plan.iter().map(|c| c.part).collect();
        assert_eq!(parts, [Part::Header, Part::Descriptor, Part::Profile]);
        let sizes: Vec<usize> = plan.iter().map(|c| c.bytes.len()).collect();
        assert_eq!(sizes, [0x1C, 8, 0xF848]);
        let joined: Vec<u8> = plan.iter().flat_map(|c| c.bytes.iter().copied()).collect();
        assert_eq!(joined.len(), FILE_LEN);
        assert_eq!(joined, save().to_bytes());
        assert_eq!(&plan[0].bytes[..4], b"20CM");
        assert_eq!(&plan[1].bytes[..4], &0x10Du32.to_le_bytes());
        assert_eq!(&plan[1].bytes[4..], &0xF848u32.to_le_bytes());
    }

    #[test]
    fn the_plan_recomputes_the_checksums_and_the_md5() {
        let mut s = save();
        let mut settings = s.settings().unwrap();
        settings.cash = 77;
        s.set_settings(&settings).unwrap();
        let plan = write_plan(&s);
        let joined: Vec<u8> = plan.iter().flat_map(|c| c.bytes.iter().copied()).collect();
        assert!(ProfileSave::parse(&joined).unwrap().integrity().all_ok());
    }

    #[test]
    fn a_new_alias_writes() {
        assert_eq!(WriteOutcome::resolve(&steps()), WriteOutcome::Ok);
    }

    #[test]
    fn an_existing_alias_asks_and_no_cancels() {
        let exists = WriteSteps { dir_exists: true, ..steps() };
        assert_eq!(WriteOutcome::resolve(&exists), WriteOutcome::Cancelled);
        assert_eq!(WriteOutcome::resolve(&WriteSteps { overwrite_confirmed: true, ..exists }), WriteOutcome::Ok);
    }

    #[test]
    fn a_background_autosave_overwrites_without_asking() {
        let silent = WriteSteps { dir_exists: true, interactive: false, ..steps() };
        assert_eq!(WriteOutcome::resolve(&silent), WriteOutcome::Ok);
    }

    #[test]
    fn only_a_failed_open_is_reported() {
        let no_open = WriteSteps { open_ok: false, ..steps() };
        assert_eq!(WriteOutcome::resolve(&no_open), WriteOutcome::OpenFailed);
        assert_eq!(WriteOutcome::OpenFailed.code(), Some(7));
        assert_eq!(WriteOutcome::Ok.code(), None);
        // A full disk or a short write after the open is not an error.
        let full_disk = WriteSteps { write_error: true, ..steps() };
        assert_eq!(WriteOutcome::resolve(&full_disk), WriteOutcome::Ok);
    }

    #[test]
    fn a_cancelled_overwrite_comes_before_the_open_failure() {
        let both = WriteSteps { dir_exists: true, open_ok: false, ..steps() };
        assert_eq!(WriteOutcome::resolve(&both), WriteOutcome::Cancelled);
    }
}

//! The phone: the SMS inbox state machine, delivery and the sender metadata
//! (`docs/specs/career-phone.md`, `docs/specs/career-text-2.md` section 1).
//!
//! - [`ids`]: the immediate and non-story id sets, label names and the sender table (numeric metadata only: the
//!   texts are loaded from the install language table at run time by label).
//! - [`queue`]: `EAddSMS`, the pending list, the per-update flush under [`FlushContext`] and the `ECellCall`
//!   request.
//! - [`inbox`]: the two-tab list ordering and the open, replay and delete actions on [`crate::settings::SmsInbox`].
//! - [`voice`]: the `(group, line)` selector of a call id.
//! - [`bins`]: the messages each Blacklist bin sends, by moment ([`bin_messages`]).
//!
//! The slot table itself is part of [`crate::settings::CareerSettings`] (`sms`); a [`PhoneQueue`] is the runtime
//! list that waits for a free moment.

pub mod bins;
pub mod ids;
pub mod inbox;
pub mod queue;
pub mod voice;

pub use bins::{BinMoment, bin_messages};
pub use ids::{
    FIRST_DRIVE_SMS, IMPOUND_RELEASE_SMS, SENDERS, Sender, from_label_for, is_immediate, is_known, is_story, label_for,
    sender_of, subject_label_for,
};
pub use inbox::{InboxTab, ListEntry, OpenEffect, tab_of};
pub use queue::{
    AddOutcome, CellCallOutcome, CellCallRequest, FlushContext, FlushOutcome, Language, PhoneQueue, process_cell_call,
};
pub use voice::{VoiceTake, has_recorded_take, voice_take};

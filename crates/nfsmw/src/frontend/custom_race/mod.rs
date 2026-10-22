//! Custom Race: the Quick Race menu's second icon. The player picks a kind of race (the mode select), a track of that kind
//! (`Track_Select.fng`), the options of the race (`Track_Options.fng`), a car (`Car_Select.fng`, with the Showcase) and a
//! transmission, and the race starts. Spec: `docs/specs/custom-race.md`; where the exe was read:
//! `docs/provenance/custom-race.md`.
//!
//! What the screens remember between them is in [`state`]; the pieces that need no screen (the modes, the regions, the
//! options and the track list) are the other modules.

pub mod car_list;
pub mod car_select;
pub mod mode;
pub mod options;
pub mod press_start;
pub mod records;
pub mod region;
pub mod settings;
pub mod showcase;
pub mod split;
pub mod start;
pub mod state;
pub mod track_options;
pub mod track_select;
pub mod tracks;

pub use mode::Mode;
pub use region::Region;

//! The backends of the interfaces the game does not implement yet (`nfsmw.career`, `nfsmw.races`). They are offered, so a
//! mod finds the table, and every call answers `BB_ERR_UNSUPPORTED` through the defaults of the traits.

use nfsmw_modding::host::{CareerBackend, RacesBackend};

/// The backend with no state behind it.
pub struct Unsupported;

impl CareerBackend for Unsupported {}

impl RacesBackend for Unsupported {}

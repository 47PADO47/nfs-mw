//! The backend of the interface the game does not implement yet (`nfsmw.races`). It is offered, so a mod finds the
//! table, and every call answers `BB_ERR_UNSUPPORTED` through the defaults of the trait.

use nfsmw_modding::host::RacesBackend;

/// The backend with no state behind it.
pub struct Unsupported;

impl RacesBackend for Unsupported {}

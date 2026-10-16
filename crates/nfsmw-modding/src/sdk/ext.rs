//! The NFS:MW interfaces, reached from a mod's `Ctx`.

use blackbox_modding::sdk::Ctx;

use super::ffi;
use super::{Career, Pursuit, Races};
use crate::abi::{
    IFACE_CAREER, IFACE_CAREER_VERSION, IFACE_PURSUIT, IFACE_PURSUIT_VERSION, IFACE_RACES, IFACE_RACES_VERSION,
};

/// `ctx.nfsmw_pursuit()`, `ctx.nfsmw_career()` and `ctx.nfsmw_races()`. Each returns `None` when the host does not offer the
/// interface at this version, so a mod built for NFS:MW can run without it, and must cope with `None` for an optional one.
pub trait NfsmwExt {
    /// The pursuit interface (`nfsmw.pursuit`).
    fn nfsmw_pursuit(&self) -> Option<Pursuit>;
    /// The career interface (`nfsmw.career`).
    fn nfsmw_career(&self) -> Option<Career>;
    /// The race interface (`nfsmw.races`).
    fn nfsmw_races(&self) -> Option<Races>;
}

impl NfsmwExt for Ctx {
    fn nfsmw_pursuit(&self) -> Option<Pursuit> {
        let iface = self.query_interface(IFACE_PURSUIT, IFACE_PURSUIT_VERSION.0, IFACE_PURSUIT_VERSION.1)?;
        Some(Pursuit::new(ffi::pursuit::read(&iface)))
    }

    fn nfsmw_career(&self) -> Option<Career> {
        let iface = self.query_interface(IFACE_CAREER, IFACE_CAREER_VERSION.0, IFACE_CAREER_VERSION.1)?;
        Some(Career::new(ffi::career::read(&iface)))
    }

    fn nfsmw_races(&self) -> Option<Races> {
        let iface = self.query_interface(IFACE_RACES, IFACE_RACES_VERSION.0, IFACE_RACES_VERSION.1)?;
        Some(Races::new(ffi::races::read(&iface)))
    }
}

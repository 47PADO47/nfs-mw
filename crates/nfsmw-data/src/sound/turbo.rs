//! The `turbosfx` class. Rules: `docs/specs/engine-sound-effects.md` §4.

use super::fields::Fields;

/// The sound of a turbocharger or supercharger.
#[derive(Debug, Clone, PartialEq)]
pub struct TurboSound {
    /// The collection's name or hash.
    pub name: String,
    /// The bank in `SOUND/TURBO` (`TURBO_TUN_SML_0_MB.abk`).
    pub bank: String,
    /// Spool loop volume, 0 to 32767.
    pub spool_volume: u32,
    /// Updates of full throttle to charge the spool.
    pub charge_time: f32,
    /// Charge lost per update off the throttle.
    pub leak_rate: f32,
    /// Blow-off volumes (the short and the two long samples), 0 to 32767; 0 means none.
    pub blowoff_volume: [u32; 2],
}

impl TurboSound {
    /// Reads a `turbosfx` collection; `None` for the `default` one, which switches the turbo sound off.
    pub(super) fn read(c: Fields<'_>) -> Option<Self> {
        if c.0.key() == blackbox_attrib::vlt_hash("default") {
            return None;
        }
        Some(Self {
            name: c.name(),
            bank: c.string("BankName"),
            spool_volume: c.u32("Vol_Spool"),
            charge_time: c.f32("ChargeTime"),
            leak_rate: c.f32("Leak_Rate"),
            blowoff_volume: [c.u32("Vol_Blowoff1"), c.u32("Vol_Blowoff2")],
        })
    }
}

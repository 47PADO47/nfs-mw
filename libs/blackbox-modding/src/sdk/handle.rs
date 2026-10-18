//! Typed handles to objects the host owns (ADR 0005 section 4). Each is a `Copy` wrapper of the raw `u64`. The host checks
//! every use: a handle to an object that no longer exists returns `Error::Stale`.
//!
//! - `VehicleId`, `WorldId`, `MenuId`, `AbilityId`: named by events and vehicle calls.
//! - `ActionId`: an input action (a hotkey a mod registered, or a vanilla action).
//! - `CommandId`: a console command the mod registered.
//! - `BlipId`: a map blip the mod added.
//! - `SoundId`: a one-shot sound the mod started.
//! - `UiThemeId`: a theme the mod registered.

macro_rules! handle {
    ($($name:ident),* $(,)?) => {
        $(
            /// A handle the host gave. See the module documentation.
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub struct $name(u64);

            impl $name {
                /// Wrap a raw handle value the host gave.
                pub const fn from_raw(raw: u64) -> Self {
                    $name(raw)
                }

                /// The raw handle value, for logging or for a host that takes raw values.
                pub const fn raw(self) -> u64 {
                    self.0
                }
            }
        )*
    };
}

handle!(VehicleId, WorldId, MenuId, AbilityId, ActionId, CommandId, BlipId, SoundId, UiThemeId);

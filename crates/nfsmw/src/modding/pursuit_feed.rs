//! The chase as the mods see it: the snapshot the world publishes in its police update and the notices the mods hear
//! when it changes (both in `scenes/world/feed/pursuit.rs`, which the HUD reads too), and the requests the mods make of
//! the chase.

use nfsmw_modding::abi::EndMode;

pub use crate::scenes::world::feed::pursuit::{Notice, PursuitView, observe};

/// A change a mod asks of the chase. The world applies them in its police update, in order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PursuitRequest {
    /// Set the heat of the running chase. The world keeps it within the chase's range.
    SetHeat(f32),
    /// End the running chase the way `mode` says.
    End(EndMode),
}

//! The trait a mod implements.

use super::{Ctx, Event, ModResult};

/// A mod. The SDK calls `load` once after the handshake, `event` for each event the mod receives, and `unload`
/// once before the library is closed.
///
/// Frame and tick hooks are events too (`FrameStart`, `SceneTick`); there is no separate tick method.
pub trait Mod: Sized {
    /// Runs once after the handshake. Register commands and subscriptions here. An `Err` rejects the mod, and the
    /// host removes anything it registered.
    fn load(ctx: &mut Ctx) -> ModResult<Self>;

    /// Runs for each event the mod receives. The default does nothing.
    fn event(&mut self, _ctx: &mut Ctx, _event: &Event) {}

    /// Runs once before the mod is removed. The default does nothing.
    fn unload(&mut self, _ctx: &mut Ctx) {}
}

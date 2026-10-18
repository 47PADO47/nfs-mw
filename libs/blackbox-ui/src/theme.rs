//! The theme trait and the theme identifiers.

use crate::component::Component;
use crate::context::BuildCx;
use crate::slot::SlotId;
use crate::tokens::Tokens;

/// Which theme a component comes from. `Original` is the floor every other theme falls back to.
/// `Custom` (the Bevy palette), `OriginalGold` and `Remastered` are the built-in alternatives; `Mod(n)` is a theme a mod
/// registered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThemeId {
    Original,
    Custom,
    OriginalGold,
    Remastered,
    Mod(u32),
}

/// A set of slot overrides. A theme implements only the slots it replaces and returns `None` for the rest;
/// the registry then asks the Original theme.
///
/// `Send + Sync` because the registry lives in a shared resource.
pub trait Theme: Send + Sync {
    fn id(&self) -> ThemeId;

    /// The component for `slot`, or `None` to fall back to the Original theme.
    fn element(&self, slot: SlotId, cx: &BuildCx<'_>) -> Option<Box<dyn Component>>;

    /// The colours, corner radius and padding this theme declares. The registry writes them into the skin while the
    /// theme is active (see [`Tokens`]). A theme that declares none keeps the neutral default.
    fn tokens(&self) -> Tokens {
        Tokens::default()
    }
}

//! The HUDs of a split-screen race: the four packages of `GLOBAL/InGameSplitScreen.bun` (`HUD_Player1.fng`,
//! `HUD_Player2.fng` and the two drag ones), one per player, bound to the player's [`HudState`](super::HudState) and drawn
//! over their half of the screen. Spec: `docs/specs/split-screen-hud.md`.
//!
//! - `package`: which package and texture pack a race kind and a player take.
//! - `seat`, `tach`, `show`: one package bound to a state (gauges, race readout, what shows).
//! - `place`: where it lands in the window. `map`: the shared minimap of the first player's package.
//! - `pair`, `huds`, `plugin`: the two HUDs of a kind in one runtime, the app's resource, and the Bevy system.
//! - `pursuit`: the strip of a chase drawn over each half (heat pips, busted and cool-down bars).

mod huds;
#[cfg(test)]
mod install_tests;
mod map;
mod names;
mod package;
mod pair;
mod place;
mod plugin;
mod pursuit;
#[cfg(test)]
mod raster_tests;
mod seat;
mod show;
mod tach;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use huds::SplitHuds;
pub use package::{Kind, SPLIT_FILE, Seat};
#[allow(unused_imports)] // for the app that draws the world's two pictures
pub use place::half_rect;
pub(super) use plugin::{install, load};

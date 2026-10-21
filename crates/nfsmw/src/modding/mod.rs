//! Native mods (ADR 0005): the game's side of the mod host.
//!
//! The loader and the dispatcher live in `blackbox-modding` (feature `host`). This module decides which mods may load
//! ([`policy`], from the `mods` setting and `--no-mods`), starts them and adds their `files/` folders to the install
//! ([`open_install`]), sends the frame events ([`ModsPlugin`]), and answers the `mods` console command ([`listing`]).
//! The game offers its interfaces, in [`backends`]: console commands and lines (`bb.commands`), hotkeys (`bb.input`),
//! the day clock (`bb.time`), minimap blips (`bb.map`) and the options rows (`bb.menus`), and the `nfsmw.*` ones: the
//! pursuit (`nfsmw.pursuit`), and career and races, which are offered with no state yet. The chase reaches the mods
//! through [`outbox`] and [`pursuit_feed`]: the world publishes it, the events go out at `ModsPost`. The mods get the log
//! and the events as well.

mod backends;
mod console;
pub(crate) mod content;
pub(crate) mod cop_rows;
mod events;
mod layers;
pub mod manager;
mod mw_events;
mod open;
pub mod outbox;
mod plugin;
mod policy;
pub mod pursuit_feed;
pub mod races_feed;
mod runtime;

#[cfg(test)]
mod hello_tests;
#[cfg(test)]
mod tests;

pub use backends::menus::{RowInfo, change as change_menu_row, rows as menu_rows};
pub use backends::ui::{
    any_theme as has_ui_themes, chosen_display as ui_theme_display, step_choice as step_ui_theme_choice,
};
pub use console::{has_command, help_entries, listing, run_command};
pub use open::open_install;
pub use plugin::ModsPlugin;
pub use runtime::Mods;

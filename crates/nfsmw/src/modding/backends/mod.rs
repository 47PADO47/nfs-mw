//! The game's side of the interfaces a mod can ask for: the four generic ones (`bb.commands`, `bb.input`, `bb.time` and
//! `bb.map`, and `bb.menus`) and the three `nfsmw.*` ones (`nfsmw.pursuit`, and `nfsmw.career` and `nfsmw.races` with no
//! state behind them yet).
//!
//! Each interface has a backend trait in `blackbox-modding` or `nfsmw-modding` and a port here that implements it. A call
//! from a mod runs on the main thread, inside a mod callback, and reads or writes the [`Bridge`]: the per-frame context
//! the frame systems sync with the scene and the console. The systems in `plugin.rs` apply what the mods asked for to the
//! data that owns it (the day clock on the scene, the console's line queue, the minimap blips on the scene). The chase
//! has its own outbox ([`crate::modding::outbox`]): the world publishes it and applies the requests.

mod bridge;
pub mod commands;
mod input;
mod map;
pub mod menu_store;
pub mod menus;
mod nfsmw_pursuit;
mod time;
mod unsupported;

#[cfg(test)]
mod bridge_tests;
#[cfg(test)]
mod commands_tests;
#[cfg(test)]
mod input_tests;
#[cfg(test)]
mod map_tests;
#[cfg(test)]
mod menus_tests;
#[cfg(test)]
mod nfsmw_pursuit_tests;
#[cfg(test)]
pub(crate) mod support;
#[cfg(test)]
mod time_tests;

use std::rc::Rc;

use blackbox_modding::host::{Attached, InterfaceRegistry};
use nfsmw_modding::abi::{
    IFACE_CAREER, IFACE_CAREER_VERSION, IFACE_PURSUIT, IFACE_PURSUIT_VERSION, IFACE_RACES, IFACE_RACES_VERSION,
};
use nfsmw_modding::host::{career_table, pursuit_table, races_table};

pub use bridge::Bridge;

/// The interfaces the game offers, and the bridge the frame systems drive them through.
pub struct Interfaces {
    pub registry: InterfaceRegistry,
    pub bridge: Rc<Bridge>,
}

/// The interfaces, each backed by one shared bridge. The game adds them to the registry it starts the mods with.
pub fn interfaces() -> Interfaces {
    let bridge = Rc::new(Bridge::default());
    let mut registry = InterfaceRegistry::new();
    registry
        .add_commands(Rc::new(commands::CommandsPort(bridge.clone())))
        .add_input(Rc::new(input::InputPort(bridge.clone())))
        .add_time(Rc::new(time::TimePort(bridge.clone())))
        .add_map(Rc::new(map::MapPort(bridge.clone())))
        .add_menus(Rc::new(menus::MenusPort(bridge.clone())));
    let pursuit_bridge = bridge.clone();
    registry
        .add_provider(IFACE_PURSUIT, IFACE_PURSUIT_VERSION, move |ctx| {
            let table = pursuit_table(Rc::new(nfsmw_pursuit::PursuitPort(pursuit_bridge.clone())), ctx.owner());
            Attached::from_owned(IFACE_PURSUIT, IFACE_PURSUIT_VERSION, table.header(), Box::new(table))
        })
        .add_provider(IFACE_CAREER, IFACE_CAREER_VERSION, |ctx| {
            let table = career_table(Rc::new(unsupported::Unsupported), ctx.owner());
            Attached::from_owned(IFACE_CAREER, IFACE_CAREER_VERSION, table.header(), Box::new(table))
        })
        .add_provider(IFACE_RACES, IFACE_RACES_VERSION, |ctx| {
            let table = races_table(Rc::new(unsupported::Unsupported), ctx.owner());
            Attached::from_owned(IFACE_RACES, IFACE_RACES_VERSION, table.header(), Box::new(table))
        });
    Interfaces { registry, bridge }
}

/// Whether `name` can name a command or a hotkey: not empty, and with no whitespace.
fn is_plain_name(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(char::is_whitespace)
}

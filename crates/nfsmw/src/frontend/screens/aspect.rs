//! Select the Showcase package's own aspect variant, retaining its art, bounds and opacity tracks.

use blackbox_feng::{PackageId, Runtime, runtime::INIT_SCRIPT};

use super::Screens;
use crate::frontend::flow::WIDE_ASPECT;

/// CURRENT_GEN_WIDESCREEN, also used by the installed widescreen splash (`splash.rs`).
const WIDESCREEN: u32 = 0xcb83_5ee3;
const NORMAL_GROUP: u32 = 0x1e61_7877;
const WIDE_GROUP: u32 = 0x1e61_78b9;
const HIDE: u32 = 0x0016_a259;

pub(super) fn bind(runtime: &mut Runtime, package: PackageId, name: &str, aspect: f32) {
    if !name.eq_ignore_ascii_case("Showcase.fng") {
        return;
    }
    // Both aspect messages select widescreen. Normal uses these groups' authored startup states.
    let scripts = match aspect > WIDE_ASPECT {
        true => [(NORMAL_GROUP, WIDESCREEN), (WIDE_GROUP, WIDESCREEN)],
        false => [(NORMAL_GROUP, INIT_SCRIPT), (WIDE_GROUP, HIDE)],
    };
    // The native wide responses only select these scripts. Select both atomically so an older queued
    // wide request cannot override a newer normal request before the runtime's next update.
    for (name, script) in scripts {
        if let Some(object) = runtime.find(package, name) {
            runtime.run_script(object, script);
        }
    }
}

impl Screens {
    /// Update menu geometry and the Showcase's authored variant when crossing the shared aspect threshold.
    pub fn set_aspect(&mut self, aspect: f32) {
        let changed = (self.aspect > WIDE_ASPECT) != (aspect > WIDE_ASPECT);
        self.aspect = aspect;
        if changed {
            for screen in &self.stack {
                bind(&mut self.runtime, screen.id, &screen.name, aspect);
            }
        }
    }
}

#[cfg(test)]
#[test]
#[ignore = "requires installed Showcase package; diagnostic for authored aspect messages"]
fn batch12_showcase_aspect_message_diagnostic() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let catalog = crate::ui::Catalog::load(&dir, &crate::ui::SCREEN_FILES);
    let package = catalog.find("Showcase.fng").expect("installed Showcase");
    let mut observations = Vec::new();
    for message in [0xcb83_5ee3, 0x3d4b_7270] {
        let mut rt = crate::ui::runtime();
        let id = rt.load(package.clone());
        rt.update(2.0);
        rt.post_to_package(id, message);
        // update drains queued responses after advancing scripts. A second update applies the new tracks.
        rt.update(1.0 / 60.0);
        rt.update(1.0 / 60.0);
        let tree = rt.tree(id);
        let state = [0x1e61_7877, 0x1e61_78b9].map(|name| {
            let node = tree.nodes.iter().find(|node| node.name_hash == name).expect("authored vignette parent");
            println!("aspect message {message:08x}, parent {name:08x}, local {:?}, world {:?}, position {:?}, size {:?}, visible {}",
                node.colour, node.world_colour, node.local_position, node.size, node.visible);
            node.visible && node.world_colour[3] > 0
        });
        observations.push(state);
        bind(&mut rt, id, "Showcase.fng", 4.0 / 3.0);
        rt.update(1.0 / 60.0);
        let normal = rt.tree(id);
        for (name, active) in [(NORMAL_GROUP, true), (WIDE_GROUP, false)] {
            let node = normal.nodes.iter().find(|node| node.name_hash == name).unwrap();
            assert_eq!(node.visible && node.world_colour[3] > 0, active, "authored normal state {name:08x}");
        }
    }
    assert_eq!(observations, [[false, true], [false, true]], "both installed messages select wide art");
}

#[cfg(test)]
mod tests;

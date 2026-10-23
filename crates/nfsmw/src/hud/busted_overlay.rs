//! The busted overlay: `BUSTED_OVERLAY.fng` (`GLOBAL/InGameB.bun`), the surveillance-camera look the original puts over the
//! frame after the arrest shot (`docs/specs/busted-sequence.md` section 6.1). The package has no data binding, so it runs
//! its own scripts and nothing more. The HUD loads it into its runtime while [`HudState::busted_overlay`] is set and
//! unloads it when that clears, so its fades start again on each bust.

#[cfg(test)]
use blackbox_feng::UiTree;
use blackbox_feng::package::Package;
use blackbox_feng::{PackageId, Runtime};

/// The overlay's package name in the install's HUD files.
pub const OVERLAY_PACKAGE: &str = "BUSTED_OVERLAY.fng";

/// The overlay's package, and the id it has in the runtime while it is up.
pub struct BustedOverlay {
    package: Package,
    shown: Option<PackageId>,
}

impl BustedOverlay {
    pub fn new(package: Package) -> Self {
        Self { package, shown: None }
    }

    /// Loads the overlay while `shown` is set and unloads it when that clears. Nothing happens when it is already in
    /// the state asked for.
    pub fn set_shown(&mut self, runtime: &mut Runtime, shown: bool) {
        if shown == self.shown.is_some() {
            return;
        }
        match self.shown.take() {
            Some(id) => runtime.unload(id),
            None => self.shown = Some(runtime.load(self.package.clone())),
        }
    }

    /// The overlay's package, while it is up.
    pub fn shown(&self) -> Option<PackageId> {
        self.shown
    }

    /// The overlay's tree, while it is up.
    #[cfg(test)]
    pub fn tree(&self, runtime: &Runtime) -> Option<UiTree> {
        self.shown.map(|id| runtime.tree(id))
    }
}

#[cfg(test)]
mod tests {
    use blackbox_feng::package::{ObjectData, ObjectDef, ObjectKind, Package};
    use blackbox_feng::{Runtime, fe_hash_upper};

    use super::*;

    /// A package with one group under the root: enough for the runtime to hold and tree it.
    fn package() -> Package {
        let mut words = vec![0u32; 17];
        for word in words.iter_mut().take(4) {
            *word = 255;
        }
        words[13] = 1.0f32.to_bits();
        let group = ObjectDef {
            kind: ObjectKind::Group,
            guid: 1,
            name_hash: fe_hash_upper("BustedOverlay"),
            flags: 0,
            resource: None,
            parent: None,
            data: ObjectData { words },
            string: None,
            multi: None,
            scripts: Vec::new(),
            responses: Vec::new(),
        };
        Package {
            name: OVERLAY_PACKAGE.into(),
            file_name: OVERLAY_PACKAGE.into(),
            version: 0,
            resources: Vec::new(),
            objects: vec![group],
            button_count: 0,
            responses: Vec::new(),
            targets: Vec::new(),
        }
    }

    #[test]
    fn showing_loads_the_overlay_and_hiding_unloads_it() {
        let mut runtime = Runtime::new();
        let mut overlay = BustedOverlay::new(package());
        assert!(overlay.tree(&runtime).is_none(), "hidden at first");
        overlay.set_shown(&mut runtime, true);
        assert!(overlay.shown.is_some());
        assert!(overlay.tree(&runtime).is_some_and(|tree| !tree.nodes.is_empty()));
        overlay.set_shown(&mut runtime, false);
        assert!(overlay.shown.is_none());
        assert!(overlay.tree(&runtime).is_none());
    }

    #[test]
    fn asking_for_the_same_state_again_changes_nothing() {
        let mut runtime = Runtime::new();
        let mut overlay = BustedOverlay::new(package());
        overlay.set_shown(&mut runtime, true);
        let first = overlay.shown;
        overlay.set_shown(&mut runtime, true);
        assert_eq!(overlay.shown, first, "no second load");
        overlay.set_shown(&mut runtime, false);
        overlay.set_shown(&mut runtime, false);
        assert!(overlay.shown.is_none());
    }
}

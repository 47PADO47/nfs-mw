//! The shared cells the Original adapters read the FEng trees from.
//!
//! The producer rule (ADR 0008, `docs/specs/ui-themes.md` section 8): the owning plugin advances its FEng package
//! (scripts, messages) and calls `Runtime::tree` once per frame; the adapters only read. `Component::build` takes no
//! runtime and no tree (`docs/specs/ui-themes.md` section 3, decision 2 of the plan), and `LayoutCx` carries only the
//! skin and the clock. So the plugin puts each frame's tree into a cell, and an adapter copies its nodes out of the
//! cell when it is built.
//!
//! There are two cells, one per kind of producer:
//!
//! - the **HUD cell** ([`FengSource`]): one tree per HUD package (`HUD_SingleRace.fng`, the chyron, the busted overlay).
//!   The HUD adapters read it by source and copy the nodes under their root groups (`subtree.rs`).
//! - the **menu cell** (`SlotId`): one whole tree per front-end or career screen, keyed by the menu slot the screen is
//!   drawn for (`docs/specs/ui-themes-menus.md`). The menu adapters copy the whole tree. The frontend plugin fills it
//!   from the trees the presenter gets, one per screen.
//!
//! Why `Arc<Mutex<..>>` and not `Rc<RefCell<..>>`: `Theme` is `Send + Sync` (`blackbox_ui::Theme`), and the registry
//! lives in a Bevy resource, so every value a theme holds must be thread-safe. The lock is taken once per adapter per
//! build, which is a handful of times a frame.
//!
//! The cells are shared by clone: `build_themes` gives the Original theme one clone, and `UiThemes::frame` gives the
//! plugin another. Every adapter reads whatever the plugin stored last, so an adapter resolved once keeps reading the
//! current frame.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use blackbox_ui::{SlotId, UiTree};

/// Which FEng package a tree came from. Each package is its own runtime package with its own object indices, so each
/// has its own tree and its own slot in the cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FengSource {
    /// `HUD_SingleRace.fng`: the HUD gauges, the minimap, the blips, the pursuit board and the speed breaker.
    Hud,
    /// `EA_TRAX.fng`: the EA Trax chyron, the Original `RadioCard`. Drawn before the HUD (`hud/plugin.rs`).
    Chyron,
    /// `BUSTED_OVERLAY.fng`: the busted overlay. Drawn after the HUD, in the frame's full screen.
    BustedOverlay,
}

/// The number of [`FengSource`] variants, and so of slots in the HUD cell.
const SOURCES: usize = 3;

impl FengSource {
    fn index(self) -> usize {
        match self {
            FengSource::Hud => 0,
            FengSource::Chyron => 1,
            FengSource::BustedOverlay => 2,
        }
    }
}

/// The trees for the current frame: one per HUD source, and one per menu slot. A source or slot with no tree draws
/// nothing.
#[derive(Clone, Default)]
pub struct FrameCell {
    trees: Arc<Mutex<[Option<Arc<UiTree>>; SOURCES]>>,
    menus: Arc<Mutex<HashMap<SlotId, Arc<UiTree>>>>,
}

impl FrameCell {
    /// An empty cell: every source and every menu slot is absent until the plugin sets it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores `tree` as this frame's tree for `source`.
    ///
    /// The plugin calls this once per frame, after `Runtime::tree` and after the same transforms the presenter gets
    /// (the HUD viewport for `Hud`, the chyron placement for `Chyron`). The adapters then copy what the presenter
    /// would draw. Wiring is T12; T7 only defines the setter.
    // wired into the HUD plugin in T12: until then nothing fills the cell.
    #[allow(dead_code)]
    pub(crate) fn set(&self, source: FengSource, tree: UiTree) {
        self.slots()[source.index()] = Some(Arc::new(tree));
    }

    /// Empties `source` for this frame, for a package that is not drawn now (the chyron is off, the overlay is
    /// hidden). Its adapters then emit nothing.
    // wired into the HUD plugin in T12, with `set`.
    #[allow(dead_code)]
    pub(crate) fn clear(&self, source: FengSource) {
        self.slots()[source.index()] = None;
    }

    /// Stores `tree` as this frame's tree for the menu slot `slot`: the whole tree of the screen the slot is drawn for,
    /// as the presenter gets it (prompts decorated). Replaces the tree the slot had.
    ///
    /// The frontend plugin calls this once per frame for each screen on the stack that maps to the slot, and calls
    /// [`Self::clear_menu`] for every other menu slot (`docs/specs/ui-themes-menus.md`, section 3). Wiring is T12.
    // wired into the frontend plugin in T12: until then nothing fills the menu cell.
    #[allow(dead_code)]
    pub(crate) fn set_menu(&self, slot: SlotId, tree: Arc<UiTree>) {
        self.menus_lock().insert(slot, tree);
    }

    /// Empties the menu slot `slot` for this frame. Its adapter then emits nothing.
    // wired into the frontend plugin in T12, with `set_menu`.
    #[allow(dead_code)]
    pub(crate) fn clear_menu(&self, slot: SlotId) {
        self.menus_lock().remove(&slot);
    }

    /// This frame's tree for `source`, if the plugin set one.
    pub(super) fn get(&self, source: FengSource) -> Option<Arc<UiTree>> {
        self.slots()[source.index()].clone()
    }

    /// This frame's tree for the menu slot `slot`, if the plugin set one.
    pub(super) fn get_menu(&self, slot: SlotId) -> Option<Arc<UiTree>> {
        self.menus_lock().get(&slot).cloned()
    }

    /// The HUD lock. A poisoned lock still holds a valid cell (each write replaces one whole slot), so the poison is
    /// ignored rather than turned into a panic inside a draw.
    fn slots(&self) -> MutexGuard<'_, [Option<Arc<UiTree>>; SOURCES]> {
        self.trees.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The menu lock. Same rule as [`Self::slots`]: each write replaces one whole entry, so a poisoned map is still valid.
    fn menus_lock(&self) -> MutexGuard<'_, HashMap<SlotId, Arc<UiTree>>> {
        self.menus.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use blackbox_ui::{SlotId, UiTree};

use super::structure::Structure;

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
    /// Counts the changes to the cell: a layer composed from the cell at one generation is the layer for any later
    /// frame at the same generation (see [`FrameCell::generation`]).
    generation: Arc<AtomicU64>,
    /// What the HUD tree says about its own shape (`structure.rs`), worked out once per package.
    structure: Arc<Mutex<Option<Structure>>>,
    /// The LAN screens of this frame: not FEng trees, so they have their own cell, shared the same way.
    lan: crate::ui::theme::LanCell,
}

impl std::fmt::Debug for FrameCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameCell").finish_non_exhaustive()
    }
}

impl FrameCell {
    /// An empty cell: every source and every menu slot is absent until the plugin sets it.
    pub fn new() -> Self {
        Self::default()
    }

    /// The LAN screens this frame. The LAN front end fills it (it is a clone: all clones share one), the `LanMenu` and
    /// `LanHud` components read it.
    pub fn lan(&self) -> crate::ui::theme::LanCell {
        self.lan.clone()
    }

    /// Stores `tree` as this frame's tree for `source`.
    ///
    /// The plugin calls this once per frame, after `Runtime::tree` and after the same transforms the presenter gets
    /// (the HUD viewport for `Hud`, the chyron placement for `Chyron`). The adapters then copy what the presenter
    /// would draw.
    #[cfg(test)]
    pub(crate) fn set(&self, source: FengSource, tree: UiTree) {
        self.slots()[source.index()] = Some(Arc::new(tree));
        self.bump();
    }

    /// Lets `build` rewrite this frame's tree for `source` in place, so the node list and the text buffers of the last
    /// frame are reused. Starts from an empty tree when the source had none. Use it instead of [`Self::set`] when the
    /// tree is rebuilt, and skip both when it is not: an untouched cell keeps its [`generation`](Self::generation).
    pub(crate) fn fill(&self, source: FengSource, build: impl FnOnce(&mut UiTree)) {
        {
            let mut slots = self.slots();
            let tree = slots[source.index()].get_or_insert_with(|| Arc::new(UiTree::default()));
            // An adapter holds the tree only while it builds, so it is not shared here and nothing is copied.
            build(Arc::make_mut(tree));
        }
        self.bump();
    }

    /// The number of changes made to the cell so far. Reading the same number twice means no tree was set, filled or
    /// cleared in between, so anything composed from the cell is still current.
    pub(crate) fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    fn bump(&self) {
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Runs `read` on the cached shape of `tree`, the HUD package's tree as the cell holds it. The shape is worked out
    /// again when the tree is not the one it was worked out for (another package, or none yet).
    pub(super) fn with_structure<R>(&self, tree: &UiTree, read: impl FnOnce(&mut Structure) -> R) -> R {
        let generation = self.generation();
        let mut cached = self.structure.lock().unwrap_or_else(PoisonError::into_inner);
        let current = cached.take().filter(|s| s.verified == generation || s.matches(tree));
        let mut structure = current.unwrap_or_else(|| Structure::new(tree));
        structure.verified = generation;
        let result = read(&mut structure);
        *cached = Some(structure);
        result
    }

    /// Empties `source` for this frame, for a package that is not drawn now (the chyron is off, the overlay is
    /// hidden). Its adapters then emit nothing.
    pub(crate) fn clear(&self, source: FengSource) {
        let had = self.slots()[source.index()].take().is_some();
        if had {
            self.bump();
        }
    }

    /// Stores `tree` as this frame's tree for the menu slot `slot`: the whole tree of the screen the slot is drawn for,
    /// as the presenter gets it (prompts decorated). Replaces the tree the slot had.
    ///
    /// The frontend plugin calls this once per frame for each screen on the stack that maps to the slot, and calls
    /// [`Self::clear_menu`] for every other menu slot (`docs/specs/ui-themes-menus.md`, section 3).
    pub(crate) fn set_menu(&self, slot: SlotId, tree: Arc<UiTree>) {
        {
            let mut menus = self.menus_lock();
            // The same tree again (a screen that did not change) leaves the cell, and its generation, alone.
            if menus.get(&slot).is_some_and(|kept| Arc::ptr_eq(kept, &tree)) {
                return;
            }
            menus.insert(slot, tree);
        }
        self.bump();
    }

    /// Empties the menu slot `slot` for this frame. Its adapter then emits nothing.
    pub(crate) fn clear_menu(&self, slot: SlotId) {
        let had = self.menus_lock().remove(&slot).is_some();
        if had {
            self.bump();
        }
    }

    /// This frame's tree for `source`, if the plugin set one.
    pub(crate) fn get(&self, source: FengSource) -> Option<Arc<UiTree>> {
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

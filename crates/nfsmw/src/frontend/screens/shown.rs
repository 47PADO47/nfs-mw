//! The trees the stack shows, kept from one frame to the next.
//!
//! A screen's tree is its package's tree with the footer hints drawn on it. The runtime counts the changes to a package
//! (`Runtime::revision`), and the hints are a function of the screen's name, its footer, the bindings, the input device,
//! the custom font state (the hint widths are laid out with the fonts) and, on the main menu, whether a profile is
//! loaded (`prompts::hint_set`). Nothing else reaches the tree, so while that key holds, the decorated tree of the last
//! frame is still the right one and is handed on by cloning its `Arc` instead of being built again.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use blackbox_feng::{PackageId, UiTree};
use glam::Vec2;

use super::{Screen, Screens};
use crate::career::CareerHandle;
use crate::frontend::logic::Footer;
use crate::frontend::prompts::{self, HintArea};
use crate::input::{Action, Bindings, InputDevice};
use crate::settings::custom_fonts;
use crate::ui::theme::remastered::menu::layout as menu_layout;
use crate::ui::theme::{ElementId, MenuList};

/// Numbers the trees that were built, across all screens, so a number names one tree's content for good (an `Arc` can be
/// rebuilt in place at the same address, a number is never used twice).
static SERIAL: AtomicU64 = AtomicU64::new(1);

/// One stack screen's tree for the frame, with the menu slot it is drawn for.
pub struct Presented {
    /// The decorated tree. The same `Arc` as last frame's while the screen did not change.
    pub tree: Arc<UiTree>,
    /// Names the content of `tree` (see [`SERIAL`]): equal numbers mean equal trees.
    pub serial: u64,
    pub slot: Option<ElementId>,
    /// The screen's entries, for a theme that lays them out itself (icon menus only).
    pub list: Option<MenuList>,
    /// The screen's file name.
    pub name: String,
}

/// What a decorated tree was built from, apart from the bindings.
#[derive(Clone, Copy, PartialEq)]
struct Key {
    package: PackageId,
    revision: u64,
    footer: Footer,
    device: InputDevice,
    fonts: custom_fonts::Active,
    signed_in: bool,
}

/// The last decorated tree of a screen, and what it was built from.
#[derive(Default)]
pub(super) struct Cache {
    key: Option<Key>,
    bindings: Option<Bindings>,
    pub tree: Arc<UiTree>,
    /// The hints of the tree that a click presses.
    pub hints: Vec<HintArea>,
    pub serial: u64,
    /// How many times the tree was built (the tests count them).
    #[cfg(test)]
    builds: u64,
}

impl Cache {
    /// Whether the tree was built from `key` and `bindings`.
    fn is_current(&self, key: &Key, bindings: &Bindings) -> bool {
        self.key.as_ref() == Some(key) && self.bindings.as_ref().is_some_and(|kept| kept.0 == bindings.0)
    }
}

/// A screen's cache. The tree is looked up through `&Screens` (the frame's systems only read the stack), so it is behind
/// a lock that only that one caller takes.
#[derive(Default)]
pub(super) struct Shown {
    cache: Mutex<Cache>,
}

impl Shown {
    pub(super) fn lock(&self) -> MutexGuard<'_, Cache> {
        // Each write replaces whole fields, so a poisoned cache is still one that rebuilds on a key it does not hold.
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Screens {
    /// The screen's decorated tree, built again only when what it was built from changed.
    fn shown<'s>(&self, screen: &'s Screen, bindings: &Bindings, device: InputDevice) -> MutexGuard<'s, Cache> {
        let key = Key {
            package: screen.id,
            revision: self.runtime.revision(screen.id),
            footer: screen.logic.footer(),
            device,
            fonts: custom_fonts::active(),
            signed_in: CareerHandle::shared().is_loaded(),
        };
        let mut guard = screen.shown.lock();
        if guard.is_current(&key, bindings) {
            return guard;
        }
        let cache = &mut *guard;
        // The frame cell may still hold the last tree; then it is left alone and this one is built from scratch.
        if Arc::get_mut(&mut cache.tree).is_none() {
            cache.tree = Arc::new(UiTree::default());
        }
        let tree = Arc::get_mut(&mut cache.tree).expect("a tree nobody else holds");
        self.runtime.tree_into(screen.id, tree);
        cache.hints = prompts::decorate(tree, &screen.name, key.footer, bindings, device, &self.assets);
        cache.key = Some(key);
        match cache.bindings.as_mut() {
            Some(kept) => kept.0.clone_from(&bindings.0),
            None => cache.bindings = Some(bindings.clone()),
        }
        cache.serial = SERIAL.fetch_add(1, Ordering::Relaxed);
        #[cfg(test)]
        {
            cache.builds += 1;
        }
        guard
    }

    /// The trees to draw, lowest screen first.
    #[cfg(test)]
    pub fn trees(&self) -> Vec<UiTree> {
        self.stack[self.visible_start()..].iter().map(|s| self.runtime.tree(s.id)).collect()
    }

    /// The trees to draw this frame, lowest first, each with its menu slot. The decorated trees are what the presenter
    /// gets, so the menu cell (`ui/theme/original/frame.rs`) holds the same ones. A screen that did not change since the
    /// last call hands over the same `Arc`.
    pub fn presented(&self, bindings: &Bindings, device: InputDevice) -> Vec<Presented> {
        let mut presented = Vec::with_capacity(self.stack.len());
        for screen in &self.stack[self.visible_start()..] {
            let (tree, serial) = {
                let shown = self.shown(screen, bindings, device);
                (shown.tree.clone(), shown.serial)
            };
            presented.push(Presented {
                tree,
                serial,
                slot: screen.slot,
                list: self.list_of(screen, bindings, device),
                name: screen.name.clone(),
            });
        }
        presented
    }

    fn visible_start(&self) -> usize {
        self.stack.iter().rposition(|screen| screen.logic.covers_screens()).unwrap_or(0)
    }

    /// The trees alone, lowest first (the tests read them; the presenter takes [`Self::presented`]).
    #[cfg(test)]
    pub fn presented_trees(&self, bindings: &Bindings, device: InputDevice) -> Vec<UiTree> {
        self.presented(bindings, device).into_iter().map(|presented| (*presented.tree).clone()).collect()
    }

    /// How many decorated trees the screens of the stack have built (the tests compare it between frames).
    #[cfg(test)]
    pub fn tree_builds(&self) -> u64 {
        self.stack.iter().map(|screen| screen.shown.lock().builds).sum()
    }

    /// The footer hint of the top screen under a point that a click presses, as the screen draws it now.
    pub fn hint_at(&self, point: Vec2, bindings: &Bindings, device: InputDevice) -> Option<Action> {
        let top = self.stack.last()?;
        if top.logic.vertical() {
            let list = self.list_of(top, bindings, device)?;
            return menu_layout::hint_at(&list.hints, point, self.aspect);
        }
        let shown = self.shown(top, bindings, device);
        shown.hints.iter().find(|h| h.covers(point)).map(|h| h.action)
    }
}

#[cfg(test)]
mod tests {
    use bevy_input::keyboard::KeyCode;

    use super::*;

    fn key(revision: u64) -> Key {
        Key {
            package: PackageId(1),
            revision,
            footer: Footer::Standard,
            device: InputDevice::Keyboard,
            fonts: custom_fonts::active(),
            signed_in: false,
        }
    }

    fn built(key: Key, bindings: &Bindings) -> Cache {
        Cache { key: Some(key), bindings: Some(bindings.clone()), ..Cache::default() }
    }

    #[test]
    fn a_tree_is_current_for_the_key_and_bindings_it_was_built_from() {
        let bindings = Bindings::default();
        let cache = built(key(3), &bindings);
        assert!(cache.is_current(&key(3), &bindings));
        assert!(!cache.is_current(&key(4), &bindings), "the package moved");
        assert!(!Cache::default().is_current(&key(3), &bindings), "nothing was built yet");
    }

    #[test]
    fn another_footer_device_or_font_state_is_not_current() {
        let bindings = Bindings::default();
        let cache = built(key(1), &bindings);
        assert!(!cache.is_current(&Key { footer: Footer::BackOnly, ..key(1) }, &bindings));
        assert!(!cache.is_current(&Key { device: InputDevice::Xbox, ..key(1) }, &bindings));
        assert!(!cache.is_current(&Key { signed_in: true, ..key(1) }, &bindings));
        let fonts = custom_fonts::Active { enabled: !key(1).fonts.enabled, ..key(1).fonts };
        assert!(!cache.is_current(&Key { fonts, ..key(1) }, &bindings));
    }

    #[test]
    fn a_rebound_key_is_not_current() {
        let bindings = Bindings::default();
        let cache = built(key(1), &bindings);
        let mut rebound = bindings.clone();
        rebound.rebind_key(Action::MenuAccept, KeyCode::KeyQ);
        assert!(!cache.is_current(&key(1), &rebound));
    }
}

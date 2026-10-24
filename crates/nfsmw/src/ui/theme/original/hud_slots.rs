//! Which adapter each HUD slot gets, and which FEng package and roots it reads. The roots are in `hud/roots.rs`.
//!
//! Composition (`docs/specs/ui-themes.md` section 8, point 4). Every HUD node is emitted by at most one of these
//! adapters, so a composer that draws all the slots once each gets the HUD with no node twice:
//!
//! - **Tachometer and GearIndicator.** The gear digit is inside the tachometer group. The tachometer adapter excludes
//!   the gear's subtree, and the gear adapter emits it. So a theme that replaces the tachometer keeps the gear unless
//!   it also replaces the gear.
//! - **Minimap and Blips.** The blip pool objects sit inside the track group. The minimap adapter excludes them, and
//!   the blips adapter emits them.
//! - **PursuitMeter.** One slot for the whole chase display: the signed meter and its cool-down bar, plus the pursuit
//!   board (its timer and the cops count). The board is bound in the same place as the meter and shows only while a
//!   chase runs, so a split would add a slot with no consumer.
//! - **RadioCard and BustedOverlay.** Their packages have no HUD root, so each emits its whole package.
//!
//! Nodes in no slot: the rear-view mirror and the bounty row (not confirmed under the board). The police radar is the
//! `PoliceRadar` slot, with its laser; the heat arcs belong to the minimap. They are not lost. [`emit_unclaimed`] appends
//! every node of the HUD package that no HUD slot takes, so the composed HUD draws them as the package does (`docs/specs/ui-themes-compose.md`).

use std::cell::RefCell;
use std::collections::HashMap;

use blackbox_ui::DrawList;
#[cfg(test)]
use blackbox_ui::UiTree;

use crate::hud::roots;
use crate::ui::theme::ElementId;

use super::frame::{FengSource, FrameCell};
use super::structure::NONE;
use super::subtree::{FengSubtree, Scope};

/// The slots that read the HUD package, in catalogue order. The composer builds them in this order.
pub(crate) const HUD_SLOTS: [ElementId; 10] = [
    ElementId::Tachometer,
    ElementId::Speedometer,
    ElementId::GearIndicator,
    ElementId::Minimap,
    ElementId::Blips,
    ElementId::PursuitMeter,
    ElementId::Nitrous,
    ElementId::Turbo,
    ElementId::Speedbreaker,
    ElementId::PoliceRadar,
];

/// The scope of a slot in the HUD package, or `None` for a slot that reads another package.
fn hud_scope(slot: ElementId) -> Option<Scope> {
    let scope = |element: roots::Roots| Scope::Under { roots: element.roots, excluded: element.excluded };
    Some(match slot {
        ElementId::Tachometer => scope(roots::tachometer()),
        ElementId::Speedometer => scope(roots::speedometer()),
        ElementId::GearIndicator => scope(roots::gear()),
        ElementId::Minimap => scope(roots::minimap()),
        ElementId::Blips => scope(roots::blips()),
        ElementId::PursuitMeter => scope(roots::pursuit_meter()),
        ElementId::Nitrous => scope(roots::nitrous()),
        ElementId::Turbo => scope(roots::turbo()),
        ElementId::Speedbreaker => scope(roots::speedbreaker()),
        ElementId::PoliceRadar => scope(roots::police_radar()),
        _ => return None,
    })
}

/// The adapter for a HUD slot, or `None` for a slot this file does not cover (the menu slots, T8).
pub(super) fn adapter(frame: &FrameCell, slot: ElementId) -> Option<FengSubtree> {
    match slot {
        ElementId::RadioCard => Some(FengSubtree::package(frame, FengSource::Chyron)),
        ElementId::BustedOverlay => Some(FengSubtree::package(frame, FengSource::BustedOverlay)),
        _ => Some(FengSubtree::with_scope(frame, FengSource::Hud, hud_scope(slot)?)),
    }
}

/// How many HUD slot scopes take each node of `tree`, by position. Each count is 0 or 1 when the slots are disjoint
/// (`hud_slots` module docs); a count of 0 is a node no slot takes.
#[cfg(test)]
pub(crate) fn claim_counts(tree: &UiTree) -> Vec<usize> {
    let mut structure = super::structure::Structure::new(tree);
    let mut counts = vec![0; tree.nodes.len()];
    for scope in HUD_SLOTS.iter().filter_map(|slot| hud_scope(*slot)) {
        let slot = structure.owner_slot(tree, &scope);
        for (pos, count) in counts.iter_mut().enumerate() {
            *count += usize::from(structure.keeps(tree, slot, pos));
        }
    }
    counts
}

/// The scratch [`emit_unclaimed`] keeps from one frame to the next, so a frame allocates nothing for it.
struct Scratch {
    /// Where each node's index lands in `out`.
    lookup: HashMap<usize, usize, foldhash::fast::FixedState>,
    /// Which positions of the HUD tree are not taken by any slot.
    unclaimed: Vec<bool>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch {
        lookup: HashMap::with_hasher(foldhash::fast::FixedState::default()),
        unclaimed: Vec::new(),
    });
}

/// Appends to `out` every node of the HUD package that no HUD slot takes, in the package's order, with `parent` links
/// remapped to `out`'s positions. A node whose parent is not in `out` gets `None`. Does nothing when the HUD cell is
/// empty.
///
/// A node's index is kept, so the composer can put the whole list back in object order (`DrawList::sort_by_index`).
pub(crate) fn emit_unclaimed(frame: &FrameCell, out: &mut DrawList) {
    let Some(tree) = frame.get(FengSource::Hud) else { return };
    SCRATCH.with_borrow_mut(|scratch| {
        let Scratch { lookup, unclaimed } = scratch;
        frame.with_structure(&tree, |structure| {
            let mut slots = [0; HUD_SLOTS.len()];
            for (k, slot) in HUD_SLOTS.iter().enumerate() {
                slots[k] = hud_scope(*slot).map_or(NONE as usize, |scope| structure.owner_slot(&tree, &scope));
            }
            let taken = |pos: usize| slots.iter().any(|&s| s != NONE as usize && structure.keeps(&tree, s, pos));
            unclaimed.clear();
            unclaimed.extend((0..tree.nodes.len()).map(|pos| !taken(pos)));
            // Where each node's index lands in `out`. The unclaimed nodes are added last, so they win an index that a
            // slot's node also has.
            lookup.clear();
            lookup.extend(out.iter().enumerate().map(|(pos, node)| (node.index, pos)));
            let free = tree.nodes.iter().zip(unclaimed.iter()).filter(|(_, free)| **free);
            for (next, (node, _)) in (out.len()..).zip(free) {
                lookup.insert(node.index, next);
            }
            for (node, _) in tree.nodes.iter().zip(unclaimed.iter()).filter(|(_, free)| **free) {
                let mut copy = node.clone();
                copy.parent = node.parent.and_then(|parent| lookup.get(&parent).copied());
                out.push(copy);
            }
        });
    });
}

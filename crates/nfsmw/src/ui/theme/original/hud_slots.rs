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
//! Not in any slot: the rear-view mirror, the police radar and its laser, and the heat arcs. The catalogue has no slot
//! for them yet, so no adapter emits them. The composer must either give them slots (each with a consumer) or emit
//! what no slot claims. That decision is owed to T12, and `docs/specs/ui-themes.md` section 8 records it. The bounty
//! row is in no slot either, unless it sits under the pursuit board, which is not confirmed.

use crate::hud::roots;
use crate::ui::theme::ElementId;

use super::frame::{FengSource, FrameCell};
use super::subtree::FengSubtree;

/// The adapter for a HUD slot, or `None` for a slot this file does not cover (the menu slots, T8).
pub(super) fn adapter(frame: &FrameCell, slot: ElementId) -> Option<FengSubtree> {
    let hud = |scope: roots::Roots| FengSubtree::under(frame, FengSource::Hud, scope.roots, scope.excluded);
    Some(match slot {
        ElementId::Tachometer => hud(roots::tachometer()),
        ElementId::Speedometer => hud(roots::speedometer()),
        ElementId::GearIndicator => hud(roots::gear()),
        ElementId::Minimap => hud(roots::minimap()),
        ElementId::Blips => hud(roots::blips()),
        ElementId::PursuitMeter => hud(roots::pursuit_meter()),
        ElementId::Nitrous => hud(roots::nitrous()),
        ElementId::Turbo => hud(roots::turbo()),
        ElementId::Speedbreaker => hud(roots::speedbreaker()),
        ElementId::RadioCard => FengSubtree::package(frame, FengSource::Chyron),
        ElementId::BustedOverlay => FengSubtree::package(frame, FengSource::BustedOverlay),
        _ => return None,
    })
}

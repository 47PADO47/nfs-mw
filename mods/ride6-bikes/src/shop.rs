//! What a run knows about the shop: the catalogue of optional parts, and the plans of the bikes it has imported so far.
//!
//! A bike's plan is made before its geometry is built, because it says which stock parts become solids of their own: a part
//! the plan has no slot for stays in the body, the bars or the wheel, where it always was.

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use crate::build_pieces::{PIECES, mounts_of, present};
use crate::exports::Exports;
use crate::options::Catalogue;
use crate::options_manifest::Plan;
use crate::partfile::PartTable;

/// The catalogue and the collected plans.
pub struct Shop<'a> {
    pub catalogue: &'a Catalogue,
    /// The plans of the bikes imported so far (the components of all of them make the catalog file).
    pub plans: &'a Mutex<Vec<Plan>>,
}

impl Shop<'_> {
    /// The plan of the bike in `dir` (its part table `table`) imported as `car`; `None` when the catalogue offers it nothing, or
    /// its frames cannot be found (the bike is then imported as it was, and the log says why).
    pub fn plan(&self, table: &PartTable, dir: &Path, car: &str) -> Option<Plan> {
        let options = self.catalogue.bike(&table.vehicle)?;
        let frames = Exports::scan(&dir.join("glb")).and_then(|exports| mounts_of(table, &exports));
        let mounts = match frames {
            Ok(mounts) => mounts,
            Err(error) => {
                log::warn!("{car}: no shop options, the frames of the bike are not found: {error:#}");
                return None;
            }
        };
        let plan = Plan::of(car, options, &present(table), mounts);
        (!plan.slots.is_empty()).then_some(plan)
    }

    /// Keep the plan of an imported bike, for the catalog file and the part packs.
    pub fn keep(&self, plan: Plan) {
        self.plans.lock().unwrap_or_else(PoisonError::into_inner).push(plan);
    }
}

/// The pieces of the stock bike `plan` binds to the car's own solids, as the bit mask [`crate::bake::Options::split_parts`] holds
/// (bit `n` for `PIECES[n]`).
pub fn split_mask(plan: &Plan) -> u8 {
    let mut mask = 0;
    for (n, piece) in PIECES.iter().enumerate() {
        if plan.parts.iter().any(|part| part.pack.is_none() && part.id == piece.part) {
            mask |= 1 << n;
        }
    }
    mask
}

#[cfg(test)]
#[path = "shop_tests.rs"]
mod tests;

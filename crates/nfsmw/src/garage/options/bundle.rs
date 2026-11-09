//! Parts that come with a level: the vehicle's offers' `bundle` (`libs/blackbox-components/src/bundle.rs`).
//!
//! Choosing `Brakes level 2` in the shop or the panel also fits the brake discs of kit 2 the bike lists for it, and going back
//! down takes them off (or fits the lower level's). The link is the data's: a bike's offer of the level names the components that
//! go with it, whatever slot they are in (a model category, a level or a session slot), so nothing here names brakes.

use blackbox_components::Change as Bundled;
use nfsmw_data::car::customization::Selection;
use nfsmw_data::car::mods::components::{self, select};
use nfsmw_data::car::physics::PerformancePackage;

use super::VehicleOptions;
use super::change::{Edit, write};

impl VehicleOptions {
    /// Brings the parts that go with a changed choice along: `parts` and `performance` are the selection after the edit, `before`
    /// what it was. A component a slot of the vehicle does not list (its pack is missing) cannot be fitted and is left.
    pub(super) fn with_bundles(
        &self,
        before: (&Selection, PerformancePackage),
        (parts, performance): (&mut Selection, &mut PerformancePackage),
    ) {
        let Some(own) = components::of(&self.folder) else { return };
        let was = select::picks(&self.folder, own, before.0, before.1);
        let now = select::picks(&self.folder, own, parts, *performance);
        for change in own.vehicle.bundle_changes(&was, &now) {
            let (slot, component) = match &change {
                Bundled::Set { slot, pick } => (slot, pick.component.as_deref()),
                Bundled::Reset { slot } => (slot, None),
            };
            let Some(slot) = self.slot(slot) else { continue };
            let choice = match component {
                Some(id) => slot.choices.iter().find(|choice| choice.id == id),
                None => slot.choices.first(),
            };
            let Some(choice) = choice else { continue };
            let mut color = None;
            write(slot, choice, &mut Edit { parts: &mut *parts, color: &mut color, performance: &mut *performance });
        }
    }
}

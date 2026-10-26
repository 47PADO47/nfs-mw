//! Causal cost-page producers. Spec: ai-pursuit-cost-attribution.md.

use super::super::tally::PursuitCostTally;
use super::Police;

impl Police {
    /// Counts only successful attachment to a live pursuit, regardless of support-request membership.
    pub(in crate::scenes::world::ai) fn ground_deployed(&mut self, name: &str) {
        if self.level().is_none() {
            return;
        }
        let heavy_mod = nfsmw_data::car::mods::cop_kind(name) == Some(nfsmw_data::car::mods::CopKind::Heavy);
        let support = heavy_mod
            || matches!(name.to_ascii_lowercase().as_str(), "copsuv" | "copsuvl" | "copcross" | "copsporthench");
        let (count, price) = match support {
            true => (&mut self.rep.costs.support_cars, 450),
            false => (&mut self.rep.costs.patrol_cars, 250),
        };
        *count = count.saturating_add(1);
        self.rep.add_cost(price);
    }

    /// One contact episode per traffic entity; multiple colliders and sustained contact share an episode.
    pub(in crate::scenes::world::ai) fn traffic_impacts(&mut self, entities: impl IntoIterator<Item = u32>) {
        if self.level().is_none() {
            return;
        }
        let now: std::collections::BTreeSet<u32> = entities.into_iter().collect();
        let fresh = now.difference(&self.rep.traffic_contacts).count() as u32;
        self.rep.costs.traffic_hits = self.rep.costs.traffic_hits.saturating_add(fresh);
        self.rep.add_cost(fresh.saturating_mul(500));
        self.rep.traffic_contacts = now;
    }

    /// Authored property costs from actual impacts. Continuous contact does not charge each physics step.
    pub(in crate::scenes::world::ai) fn property_impacts(&mut self, impacts: &[(u32, u32)]) {
        if self.level().is_none() {
            return;
        }
        let mut now = std::collections::BTreeSet::new();
        for &(id, value) in impacts {
            if value == 0 {
                continue;
            }
            if !now.insert(id) || self.rep.prop_contacts.contains(&id) {
                continue;
            }
            self.rep.costs.property_count = self.rep.costs.property_count.saturating_add(1);
            self.rep.costs.property_value = self.rep.costs.property_value.saturating_add(value);
            self.rep.add_cost(value);
        }
        self.rep.prop_contacts = now;
    }

    pub(in crate::scenes::world::ai) fn cost_tally(&self) -> PursuitCostTally {
        self.rep.costs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deployment_classification_uses_models_and_counts_only_live_pursuits() {
        let mut police = Police::default();
        police.ground_deployed("copmidsize");
        assert_eq!(police.cost_tally(), PursuitCostTally::default());
        police.begin(1);
        police.ground_deployed("copmidsize");
        police.ground_deployed("copsporthench");
        police.ground_deployed("copsuvpatrol");
        assert_eq!((police.cost_tally().patrol_cars, police.cost_tally().support_cars), (2, 1));
        assert_eq!(police.cost_to_state(), 950);
    }

    #[test]
    fn repeated_contact_and_duplicate_primitives_count_once_but_a_later_impact_counts_again() {
        let mut police = Police::default();
        police.begin(1);
        police.traffic_impacts([7, 7]);
        police.traffic_impacts([7]);
        police.property_impacts(&[(3, 100), (3, 100), (4, 200)]);
        police.property_impacts(&[(3, 100)]);
        assert_eq!(police.cost_to_state(), 800);
        police.traffic_impacts([]);
        police.property_impacts(&[]);
        police.traffic_impacts([7]);
        police.property_impacts(&[(3, 100)]);
        assert_eq!(police.cost_to_state(), 1400);
        assert_eq!((police.cost_tally().traffic_hits, police.cost_tally().property_count), (2, 3));
        police.begin(1);
        assert_eq!(police.cost_tally(), PursuitCostTally::default());
    }

    #[test]
    fn outcome_keeps_totals_and_non_costed_objects_do_not_create_damage_counts() {
        let mut police = Police::default();
        police.begin(1);
        police.property_impacts(&[(1, 0), (2, 75)]);
        police.end(blackbox_pursuit::EndReason::Evaded);
        police.property_impacts(&[(3, 100)]);
        assert_eq!(police.cost_to_state(), 75);
        assert_eq!((police.cost_tally().property_value, police.cost_tally().property_count), (75, 1));
    }
}

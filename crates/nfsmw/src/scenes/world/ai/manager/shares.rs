//! Custom common-car allocation. Native selection keeps its original share calculation.
use super::{Census, MAX_RECORDS, Rule};

pub(super) fn limited(rules: &[Rule], name: &str) -> bool {
    rules.iter().take(MAX_RECORDS).any(|r| r.name.eq_ignore_ascii_case(name) && r.max_instances > 0)
}

struct Group<'a> {
    name: &'a str,
    max: u32,
    percent: u32,
    quota: u32,
    remainder: u64,
}

pub(super) fn quotas(rules: &[Rule], slots: u32, census: &impl Census) -> Vec<u32> {
    let mut groups: Vec<Group<'_>> = Vec::new();
    for rule in rules {
        if let Some(group) = groups.iter_mut().find(|g| g.name.eq_ignore_ascii_case(&rule.name)) {
            if rule.max_instances > 0 {
                group.max = match group.max {
                    0 => rule.max_instances,
                    max => max.min(rule.max_instances),
                };
            }
            group.percent = group.percent.max(rule.percent);
            continue;
        }
        groups.push(Group { name: &rule.name, max: rule.max_instances, percent: rule.percent, quota: 0, remainder: 0 });
    }
    let free = slots.saturating_sub(census.others());
    let mut reserved = 0u32;
    for group in &mut groups {
        if group.max == 0 {
            continue;
        }
        group.quota = group.max;
        for rule in
            rules.iter().filter(|r| r.name.eq_ignore_ascii_case(group.name) && r.max_instances > 0 && r.percent > 0)
        {
            group.quota = group.quota.min((free.saturating_mul(rule.percent) / 100).max(1));
        }
        let active = census.active(group.name);
        let missing_trailers = group.quota.saturating_sub(active).saturating_mul(census.trailer_slots(group.name));
        reserved = reserved.saturating_add(group.quota.max(active)).saturating_add(missing_trailers);
    }
    let remaining = free.saturating_sub(reserved);
    let weight: u64 = groups
        .iter()
        .filter(|g| g.max == 0)
        .map(|g| {
            u64::from(match g.percent {
                0 => 100,
                p => p,
            })
        })
        .sum();
    if let Some(weight) = std::num::NonZeroU64::new(weight) {
        let mut assigned = 0;
        for group in groups.iter_mut().filter(|g| g.max == 0) {
            let w = u64::from(match group.percent {
                0 => 100,
                p => p,
            });
            let product = u64::from(remaining) * w;
            group.quota = (product / weight.get()) as u32;
            group.remainder = product % weight.get();
            assigned += group.quota;
        }
        for _ in assigned..remaining {
            let Some(group) = groups.iter_mut().filter(|g| g.max == 0).max_by_key(|g| g.remainder) else { break };
            group.quota += 1;
            group.remainder = 0;
        }
    }
    rules.iter().map(|r| groups.iter().find(|g| g.name.eq_ignore_ascii_case(&r.name)).map_or(0, |g| g.quota)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Cars {
        truck: u32,
        others: u32,
    }
    impl Census for Cars {
        fn active(&self, name: &str) -> u32 {
            match name.eq_ignore_ascii_case("truck") {
                true => self.truck,
                false => 0,
            }
        }
        fn others(&self) -> u32 {
            self.others
        }
        fn trailer_slots(&self, name: &str) -> u32 {
            u32::from(name.eq_ignore_ascii_case("truck"))
        }
    }
    fn rule(name: &str, max: u32, percent: u32) -> Rule {
        Rule { name: name.into(), rate: 3.0, max_instances: max, percent }
    }
    #[test]
    fn common_weights_fill_space_after_reserving_rare_roots_and_future_trailers() {
        let rules = [rule("a", 0, 20), rule("b", 0, 10), rule("truck", 1, 10)];
        let before = quotas(&rules, 40, &Cars { truck: 0, others: 1 });
        assert_eq!(before, vec![25, 12, 1]);
        let after = quotas(&rules, 40, &Cars { truck: 1, others: 2 });
        assert_eq!(before, after, "the trailer becoming active must not shrink common quotas");
        assert_eq!(quotas(&rules, 1, &Cars { truck: 0, others: 1 }), vec![0, 0, 1]);
    }
    #[test]
    fn duplicate_names_share_one_quota_and_cannot_bypass_a_rare_limit() {
        let rules = [rule("a", 0, 20), rule("A", 0, 20), rule("truck", 1, 10), rule("TRUCK", 0, 10)];
        assert_eq!(quotas(&rules, 40, &Cars { truck: 0, others: 1 }), vec![37, 37, 1, 1]);
        assert!(limited(&rules, "TRUCK"));
    }
    #[test]
    fn zero_share_common_models_have_weight_100_and_rounding_fills_budget() {
        let rules = [rule("a", 0, 0), rule("b", 0, 20)];
        assert_eq!(quotas(&rules, 20, &Cars { truck: 0, others: 1 }), vec![16, 3]);
    }
}

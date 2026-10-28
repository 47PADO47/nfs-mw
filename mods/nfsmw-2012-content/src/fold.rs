//! One car per car: the vehicles of the list that are one car under two vehicle ids become one.
//!
//! The 2012 list has a few cars twice under different vehicle ids: the player's version, with the colours the shop offers,
//! and another with one colour of its own (five of them), and a Nissan Skyline R34 GT-R that is the car with the livery
//! bundles in one vehicle and the car in plain paint in another (`docs/formats/chameleon-customization.md`). Shown as two
//! cars they are the same car twice in the garage, so the vehicle that leads (the one with the liveries, then the one with
//! most colours) is the car, and the others only lend it their colours.

use crate::liveries::Group;

/// A vehicle's bundles, and the bundles of the vehicles that are the same car under another id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folded {
    pub group: Group,
    /// The bundles of the other vehicles of the car: only their colours are taken.
    pub also: Vec<u32>,
}

/// Fold the `groups` whose cars have the same name into one each, in the order of the first of them. `name` gives the name
/// of the car in a bundle (`None` for a bundle that is never folded: an unnamed one, a police car) and `colours` the count
/// of paints a bundle holds. The group with the most liveries leads, then the one with the most colours, then the earlier;
/// the bundles of the others (not their liveries) go to [`Folded::also`], the lead's own first.
pub fn fold_same_car(
    groups: Vec<Group>,
    name: &dyn Fn(u32) -> Option<String>,
    colours: &dyn Fn(u32) -> usize,
) -> Vec<Folded> {
    let rank = |group: &Group| (group.liveries.len(), colours(group.base));
    let mut out: Vec<(Option<String>, Folded)> = Vec::new();
    for group in groups {
        let key = name(group.base);
        let known = key.as_ref().and_then(|key| out.iter().position(|(other, _)| other.as_ref() == Some(key)));
        let Some(at) = known else {
            out.push((key, Folded { group, also: Vec::new() }));
            continue;
        };
        let folded = &mut out[at].1;
        if rank(&group) <= rank(&folded.group) {
            folded.also.push(group.base);
            continue;
        }
        let old = std::mem::replace(&mut folded.group, group);
        folded.also.insert(0, old.base);
    }
    out.into_iter().map(|(_, folded)| folded).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(base: u32, liveries: &[u32]) -> Group {
        Group { base, liveries: liveries.to_vec() }
    }

    /// Bundles 1 and 2 are one car, 3 and 4 have no name (a police car is never folded), 5 stands alone.
    fn name(bundle: u32) -> Option<String> {
        match bundle {
            1 | 2 => Some("Porsche".into()),
            5 => Some("Lancia".into()),
            _ => None,
        }
    }

    #[test]
    fn two_vehicles_of_one_name_are_one_car_led_by_the_one_with_most_colours() {
        let found = fold_same_car(vec![group(1, &[]), group(5, &[]), group(2, &[])], &name, &|bundle| match bundle {
            1 => 9,
            2 => 1,
            _ => 3,
        });
        assert_eq!(found.len(), 2);
        assert_eq!(found[0], Folded { group: group(1, &[]), also: vec![2] });
        assert_eq!(found[1], Folded { group: group(5, &[]), also: vec![] });
    }

    #[test]
    fn the_vehicle_with_liveries_leads_even_when_it_comes_later_or_has_fewer_colours() {
        let found = fold_same_car(vec![group(1, &[]), group(2, &[20, 21])], &name, &|bundle| 10 - bundle as usize);
        assert_eq!(found, vec![Folded { group: group(2, &[20, 21]), also: vec![1] }]);
    }

    #[test]
    fn a_tie_goes_to_the_earlier_vehicle() {
        let found = fold_same_car(vec![group(1, &[]), group(2, &[])], &name, &|_| 4);
        assert_eq!(found, vec![Folded { group: group(1, &[]), also: vec![2] }]);
    }

    #[test]
    fn bundles_without_a_name_are_never_folded() {
        let found = fold_same_car(vec![group(3, &[]), group(4, &[])], &name, &|_| 1);
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|folded| folded.also.is_empty()));
    }

    #[test]
    fn every_bundle_is_a_lead_or_lent_to_one() {
        let groups = vec![group(1, &[]), group(2, &[]), group(3, &[]), group(5, &[])];
        let found = fold_same_car(groups, &name, &|bundle| bundle as usize);
        let mut all: Vec<u32> =
            found.iter().flat_map(|f| std::iter::once(f.group.base).chain(f.also.clone())).collect();
        all.sort_unstable();
        assert_eq!(all, vec![1, 2, 3, 5]);
    }
}

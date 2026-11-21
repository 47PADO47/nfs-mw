//! The car lot's sort (`docs/specs/career-car-lot.md` section 1.2): the list of the stable's cars in slot order, sorted
//! by a comparison that only asks whether one car's unlock rank is above another's. The sort is not stable, so the order
//! of cars of one rank comes from the slot order and the two steps below.

/// Sorts `items` as the car lot does, with `above(a, b)` true when `a` must come before `b` (its rank is higher).
///
/// 1. A first pass keeps a current car and looks at the one after it: when the current car is not above it, that car is
///    moved in front of the current one, which stays current; otherwise the next car becomes current. When this pass
///    moves nothing, the list is done.
/// 2. Otherwise a merge sort runs over the list: runs of 1, 2, 4... cars are merged pairwise from the front; a merge takes
///    the left run's car only when the right run is used up or the left car is above the right one, so of two cars of one
///    rank the right run's goes first. The sort ends after a pass that made fewer than two merges.
pub fn lot_sort<T>(items: &mut Vec<T>, above: impl Fn(&T, &T) -> bool) {
    if !first_pass(items, &above) {
        return;
    }
    let mut width = 1;
    loop {
        let merges = merge_pass(items, width, &above);
        if merges < 2 {
            return;
        }
        width *= 2;
    }
}

/// Step 1. True when a car was moved.
fn first_pass<T>(items: &mut [T], above: &impl Fn(&T, &T) -> bool) -> bool {
    let mut moved = false;
    let mut current = 0;
    while current + 1 < items.len() {
        if !above(&items[current], &items[current + 1]) {
            // The next car goes in front of the current one, which is now one place further on.
            items.swap(current, current + 1);
            moved = true;
        }
        current += 1;
    }
    moved
}

/// One pass of step 2 with runs of `width` cars. Returns the number of merges (run pairs, a lone last run included).
fn merge_pass<T>(items: &mut Vec<T>, width: usize, above: &impl Fn(&T, &T) -> bool) -> usize {
    let mut source: std::collections::VecDeque<T> = std::mem::take(items).into();
    let mut merges = 0;
    while !source.is_empty() {
        let left: std::collections::VecDeque<T> = source.drain(..width.min(source.len())).collect();
        let right: std::collections::VecDeque<T> = source.drain(..width.min(source.len())).collect();
        merge(items, left, right, above);
        merges += 1;
    }
    merges
}

/// Merges two runs onto the end of `out`.
fn merge<T>(
    out: &mut Vec<T>,
    mut left: std::collections::VecDeque<T>,
    mut right: std::collections::VecDeque<T>,
    above: &impl Fn(&T, &T) -> bool,
) {
    while let Some(first) = left.front() {
        let take_left = right.front().is_none_or(|other| above(first, other));
        let next = match take_left {
            true => left.pop_front(),
            false => right.pop_front(),
        };
        out.extend(next);
    }
    out.extend(right);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sorted(ranks: &[(char, u8)]) -> String {
        let mut items = ranks.to_vec();
        lot_sort(&mut items, |a, b| a.1 > b.1);
        items.iter().map(|item| item.0).collect()
    }

    #[test]
    fn a_list_already_in_order_with_no_ties_is_left_alone() {
        assert_eq!(sorted(&[('a', 3), ('b', 2), ('c', 1)]), "abc");
    }

    #[test]
    fn two_ties_end_in_their_slot_order_after_both_steps() {
        // First pass: b moves in front of a. Merges of one: (b)(a) gives a then b (a tie takes the right run's car).
        assert_eq!(sorted(&[('a', 2), ('b', 2), ('c', 1)]), "abc");
    }

    #[test]
    fn three_ties_do_not_keep_their_slot_order() {
        // First pass: [b, c, a]. Merges of one: (b)(c) gives c, b; (a) alone. Merge of two: (c b)(a) takes a first.
        assert_eq!(sorted(&[('a', 2), ('b', 2), ('c', 2)]), "acb");
    }

    #[test]
    fn higher_ranks_always_come_first() {
        let order = sorted(&[('a', 1), ('b', 5), ('c', 3), ('d', 5), ('e', 1), ('f', 3)]);
        let rank =
            |c: char| [('a', 1), ('b', 5), ('c', 3), ('d', 5), ('e', 1), ('f', 3)].iter().find(|x| x.0 == c).unwrap().1;
        let ranks: Vec<u8> = order.chars().map(rank).collect();
        assert!(ranks.windows(2).all(|pair| pair[0] >= pair[1]), "{order}");
    }
}

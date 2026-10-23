//! The 16-bit entity ids of the server's id space (ADR 0009 section 1) for everything that is not a player car. A player
//! car's id is its `PlayerId` (0 to 15); every other replicated thing keeps the id it got when it first appeared until
//! it is gone.
//!
//! Ids are handed out round robin from [`FIRST_WORLD_ID`], skipping ids in use, so a freed id comes back only after
//! the counter has gone round the whole space (about 65 thousand appearances): a client never sees one id stand for two
//! things a tick apart. New things are numbered in the order of their world identity ([`Native`]), and the world's own
//! identities follow its deterministic step, so two servers with the same start and inputs number alike.

use std::collections::{BTreeMap, BTreeSet};

use nfsmw_sim::EntityId;

/// The first id after the player cars (`PlayerId` 0 to 15).
pub const FIRST_WORLD_ID: EntityId = 16;

/// A replicated thing by its identity in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Native {
    /// A traffic, police or roadblock car, by its entity id in the traffic world (`AiCar::key`).
    Car(u32),
    /// The police helicopter (there is at most one).
    Helicopter,
    /// A spike strip: its roadblock's id and its index in the roadblock.
    Strip(u32, u16),
    /// A prop body in motion, by its id in the prop world.
    Prop(u32),
}

/// The ids of the things the world has now.
#[derive(Debug, Clone)]
pub struct EntityTable {
    ids: BTreeMap<Native, EntityId>,
    used: BTreeSet<EntityId>,
    /// Where the round-robin search for the next id starts.
    next: EntityId,
    /// Things that found every id in use and were left out (logged once).
    overflowed: bool,
}

impl Default for EntityTable {
    fn default() -> Self {
        Self { ids: BTreeMap::new(), used: BTreeSet::new(), next: FIRST_WORLD_ID, overflowed: false }
    }
}

impl EntityTable {
    /// Follow the world: things in `present` that had no id get one (in `Native` order), things that are gone free
    /// theirs.
    pub fn refresh(&mut self, present: impl IntoIterator<Item = Native>) {
        let present: BTreeSet<Native> = present.into_iter().collect();
        let gone: Vec<(Native, EntityId)> =
            self.ids.iter().filter(|(native, _)| !present.contains(native)).map(|(n, id)| (*n, *id)).collect();
        for (native, id) in gone {
            self.ids.remove(&native);
            self.used.remove(&id);
        }
        for native in present {
            if self.ids.contains_key(&native) {
                continue;
            }
            let Some(id) = self.allocate() else {
                if !self.overflowed {
                    log::warn!("net: every entity id is in use; {native:?} and later ones are not replicated");
                }
                self.overflowed = true;
                return;
            };
            self.ids.insert(native, id);
        }
    }

    /// The id of `native`, if it has one.
    pub fn get(&self, native: Native) -> Option<EntityId> {
        self.ids.get(&native).copied()
    }

    /// How many things have an id.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// The next free id from `next` on, wrapping to [`FIRST_WORLD_ID`] after the last.
    fn allocate(&mut self) -> Option<EntityId> {
        let span = usize::from(EntityId::MAX - FIRST_WORLD_ID) + 1;
        for _ in 0..span {
            let id = self.next;
            self.next = match id == EntityId::MAX {
                true => FIRST_WORLD_ID,
                false => id + 1,
            };
            if self.used.insert(id) {
                return Some(id);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_things_take_ids_in_native_order_after_the_players() {
        let mut table = EntityTable::default();
        table.refresh([Native::Prop(9), Native::Car(3), Native::Helicopter, Native::Car(1), Native::Strip(2, 0)]);
        let ids = [Native::Car(1), Native::Car(3), Native::Helicopter, Native::Strip(2, 0), Native::Prop(9)]
            .map(|native| table.get(native));
        assert_eq!(ids, [16, 17, 18, 19, 20].map(Some));
    }

    #[test]
    fn a_thing_keeps_its_id_and_a_gone_one_frees_it_without_reusing_it_at_once() {
        let mut table = EntityTable::default();
        table.refresh([Native::Car(1), Native::Car(2)]);
        table.refresh([Native::Car(2), Native::Car(5)]);
        assert_eq!(table.get(Native::Car(1)), None);
        assert_eq!(table.get(Native::Car(2)), Some(17), "kept");
        assert_eq!(table.get(Native::Car(5)), Some(18), "not the id car 1 just gave back");
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn the_counter_wraps_and_skips_ids_in_use() {
        let mut table = EntityTable { next: EntityId::MAX, ..EntityTable::default() };
        table.refresh([Native::Car(1)]);
        assert_eq!(table.get(Native::Car(1)), Some(EntityId::MAX));
        table.refresh([Native::Car(1), Native::Car(2)]);
        assert_eq!(table.get(Native::Car(2)), Some(FIRST_WORLD_ID), "wrapped past the player ids");
    }

    #[test]
    fn a_full_table_leaves_new_things_out() {
        let mut table = EntityTable::default();
        let span = u32::from(EntityId::MAX - FIRST_WORLD_ID) + 1;
        table.refresh((0..span).map(Native::Car));
        assert_eq!(table.len(), span as usize);
        table.refresh((0..=span).map(Native::Car));
        assert_eq!(table.get(Native::Car(span)), None);
        assert_eq!(table.len(), span as usize);
    }

    #[test]
    fn two_tables_fed_alike_number_alike() {
        let (mut a, mut b) = (EntityTable::default(), EntityTable::default());
        for tick in 0..50u32 {
            let present: Vec<Native> = (tick..tick + 7).filter(|n| n % 3 != 0).map(Native::Car).collect();
            a.refresh(present.iter().copied());
            b.refresh(present.iter().rev().copied());
        }
        let ids = |table: &EntityTable| (40..57).map(|n| table.get(Native::Car(n))).collect::<Vec<_>>();
        assert_eq!(ids(&a), ids(&b));
    }
}

//! Guarded animations of a state machine: for each animation state directly in a machine, the condition that starts it
//! and the condition that ends it, read from the links between the machine's objects. Spec:
//! `docs/formats/chameleon-active-aero.md`, "Guarded animations".
//!
//! A machine's child list has, for each child, **links** (24 bytes each: two keys and a reference to another object of the
//! machine). A link with the key `684561a168000000` joins a condition and the state or machine it guards, both ways; a
//! timed transition in the chain (condition, `Wait`, state) is the number of seconds the condition has to hold first. A state
//! inside a group machine has no guard of its own: it takes the one of the group, and the group's free condition (a child
//! that links to nothing) ends it. A state that has its own condition list (key `684563f7fd6c4600`) ends by that.

use super::*;

/// The key of the link that joins a condition and what it guards.
const LINK_GUARDS: u64 = 0x6845_61a1_6800_0000;

/// Tests that all hold, or of which one is enough.
#[derive(Debug, Clone, PartialEq)]
pub struct Guard {
    pub tests: Vec<Threshold>,
    pub any: bool,
}

/// An animation state with the guards that start and end it.
#[derive(Debug, Clone, PartialEq)]
pub struct DriveSpec {
    pub name: String,
    pub animation: u64,
    pub open: Guard,
    /// Seconds the opening condition has to hold before the animation starts.
    pub open_hold: f32,
    pub close: Guard,
}

struct Link {
    key: u64,
    target: u32,
}

struct Entry {
    owner: u32,
    child: u32,
    links: Vec<Link>,
}

fn entries(block: &[u8], all: &[Record]) -> Vec<Entry> {
    let mut out = Vec::new();
    for owner in all {
        let mut at = owner.children;
        while at != 0 {
            let Some(child) = word(block, at).filter(|id| all.iter().any(|record| record.id == *id)) else { break };
            let (Some(table), Some(count)) = (word(block, at + 4), word(block, at + 8)) else { break };
            let mut links = Vec::new();
            for n in 0..(count as usize).min(8) {
                let q = table as usize + 24 * n;
                let key =
                    u64::from(word(block, q + 8).unwrap_or(0)) | (u64::from(word(block, q + 12).unwrap_or(0)) << 32);
                let target = word(block, q + 20).and_then(|r| word(block, r as usize + 8)).unwrap_or(0);
                links.push(Link { key, target });
            }
            out.push(Entry { owner: owner.id, child, links });
            at += 16;
        }
    }
    out
}

fn guard_of(block: &[u8], all: &[Record], id: u32) -> Option<Guard> {
    let record = all.iter().find(|record| record.id == id && record.kind == CONDITION)?;
    let tests = condition_of(block, record);
    let any = rows(block, record.props)
        .iter()
        .find(|row| row.key == KEY_ANY)
        .and_then(|row| text_of(block, row))
        .is_some_and(|text| text == "1");
    (!tests.is_empty()).then_some(Guard { tests, any })
}

/// The guarded animation states directly in a machine.
pub fn drives(block: &[u8]) -> Vec<DriveSpec> {
    let all = records(block);
    let links = entries(block, &all);
    let kind_of = |id: u32| all.iter().find(|record| record.id == id).map(|record| record.kind);
    let owner_of = |id: u32| links.iter().find(|entry| entry.child == id).map(|entry| entry.owner);
    let guards = |owner: u32, id: u32| -> Option<u32> {
        let entry = links.iter().find(|entry| entry.owner == owner && entry.child == id)?;
        entry.links.iter().find(|link| link.key == LINK_GUARDS && kind_of(link.target).is_some()).map(|l| l.target)
    };
    // The condition that starts `id`, and the seconds it has to hold, looking up through the groups.
    let opening = |mut id: u32| -> Option<(u32, f32)> {
        for _ in 0..6 {
            let owner = owner_of(id)?;
            if let Some(target) = guards(owner, id).filter(|t| kind_of(*t) == Some(CONDITION)) {
                let wait = guards(owner, target)
                    .and_then(|t| all.iter().find(|record| record.id == t && record.kind == TRANSITION))
                    .and_then(|record| timer_of(block, record));
                return Some((target, wait.unwrap_or(0.0)));
            }
            id = owner;
        }
        None
    };
    // The free condition of a group: a condition child that links to nothing.
    let free = |owner: u32| -> Option<u32> {
        links
            .iter()
            .filter(|entry| entry.owner == owner && kind_of(entry.child) == Some(CONDITION))
            .find(|entry| entry.links.iter().all(|link| kind_of(link.target).is_none()))
            .map(|entry| entry.child)
    };
    let mut out = Vec::new();
    for state in all.iter().filter(|record| record.kind == STATE) {
        let Some(owner) = owner_of(state.id).filter(|owner| kind_of(*owner) == Some(MACHINE)) else { continue };
        let props = rows(block, state.props);
        let Some(animation) = props
            .iter()
            .find(|row| row.key == KEY_ANIMATION)
            .and_then(|row| text_of(block, row))
            .and_then(|text| text.parse::<u64>().ok())
        else {
            continue;
        };
        let Some((start, open_hold)) = opening(state.id) else { continue };
        let Some(open) = guard_of(block, &all, start) else { continue };
        let own: Vec<Threshold> =
            props.iter().find(|row| row.key == KEY_CLOSE).map(|row| thresholds_of(block, row)).unwrap_or_default();
        let any = props
            .iter()
            .find(|row| row.key == KEY_ANY)
            .and_then(|row| text_of(block, row))
            .is_some_and(|text| text == "1");
        let close = match own.is_empty() {
            false => Some(Guard { tests: own, any }),
            true => free(owner).and_then(|id| guard_of(block, &all, id)),
        };
        let Some(close) = close else { continue };
        let name = text_at(block, state.name).unwrap_or_default();
        out.push(DriveSpec { name, animation, open, open_hold, close });
    }
    out
}

//! Which parts of a split-screen package are shown. The packages draw everything they hold at rest, including the sample
//! text of the message and the countdown, so the host hides what has no state to show: the race information without a race,
//! the nitrous gauge without a tank, the countdown and the message line between steps, the minimap without a position, and
//! the parts nothing feeds.

use blackbox_feng::{ObjectRef, PackageId, Runtime, fe_hash_upper};

use super::Kind;
use super::names::{
    CLOCK, CLOCK_ICON, COUNTDOWN, ENGINE_HEAT, GENERIC_MESSAGE, INFO_GROUP, MILESTONE, NITROUS_BAR, NITROUS_GROUP,
    SPEED_TRAP_ICON, TIME_GLOW, WRONG_WAY,
};
use crate::hud::minimap::{ARROW, BACKING, GROUP};

/// What the HUD shows this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Want {
    /// A race runs: the place and the clock show.
    pub race: bool,
    /// A countdown step is up.
    pub countdown: bool,
    /// A line of the message group is up.
    pub message: bool,
    /// The car has a nitrous tank.
    pub nitrous: bool,
    /// The minimap shows (the first player's package only).
    pub map: bool,
}

/// The subtrees each flag hides, found once.
pub struct Show {
    package: PackageId,
    parent: Vec<Option<usize>>,
    always: Vec<usize>,
    race: Vec<usize>,
    countdown: Vec<usize>,
    message: Vec<usize>,
    nitrous: Vec<usize>,
    map: Vec<usize>,
    applied: Option<Want>,
}

impl Show {
    pub fn new(rt: &Runtime, package: PackageId, kind: Kind) -> Self {
        let Some(def) = rt.package(package) else {
            return Self::empty(package);
        };
        let index_of: std::collections::HashMap<u32, usize> =
            def.objects.iter().enumerate().map(|(i, o)| (o.guid, i)).collect();
        let parent: Vec<Option<usize>> = def
            .objects
            .iter()
            .enumerate()
            .map(|(i, o)| o.parent.and_then(|g| index_of.get(&g).copied()).filter(|p| *p != i))
            .collect();
        let named = |hash: u32| -> Vec<usize> {
            def.objects.iter().enumerate().filter(|(_, o)| o.name_hash == hash).map(|(i, _)| i).collect()
        };
        let by_name = |name: &str| named(fe_hash_upper(name));
        let top = |indices: Vec<usize>| -> Vec<usize> { indices.into_iter().map(|i| top_of(&parent, i)).collect() };
        let speed_traps = def
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                let name = o.resource.and_then(|r| def.resources.get(r)).map(|r| r.name.as_str());
                name.is_some_and(|n| n.to_ascii_lowercase().starts_with(&SPEED_TRAP_ICON.to_ascii_lowercase()))
            })
            .map(|(i, _)| i);
        let always: Vec<usize> =
            [WRONG_WAY, TIME_GLOW, MILESTONE, ENGINE_HEAT].into_iter().flat_map(named).chain(speed_traps).collect();
        // A drag package keeps the gear and the shift backing in its race information, so only the clock goes.
        let race: Vec<usize> = match kind {
            Kind::Race => named(INFO_GROUP).into_iter().chain(named(CLOCK_ICON)).collect(),
            Kind::Drag => by_name(CLOCK).into_iter().chain(named(CLOCK_ICON)).collect(),
        };
        let map = top(by_name(GROUP).into_iter().chain(by_name(BACKING)).chain(by_name(ARROW)).collect());
        Self {
            package,
            always,
            race,
            countdown: by_name(COUNTDOWN),
            message: top(named(GENERIC_MESSAGE)),
            nitrous: named(NITROUS_GROUP).into_iter().chain(named(NITROUS_BAR)).collect(),
            map,
            parent,
            applied: None,
        }
    }

    fn empty(package: PackageId) -> Self {
        Self {
            package,
            parent: Vec::new(),
            always: Vec::new(),
            race: Vec::new(),
            countdown: Vec::new(),
            message: Vec::new(),
            nitrous: Vec::new(),
            map: Vec::new(),
            applied: None,
        }
    }

    /// Hides and shows the objects for `want`. Nothing happens while the request is the one already applied. Call it
    /// before the binders: they hide single objects (a leading zero, a blip) that this resets.
    pub fn update(&mut self, rt: &mut Runtime, want: Want) {
        if self.applied == Some(want) {
            return;
        }
        self.applied = Some(want);
        let mut hidden_roots: Vec<usize> = self.always.clone();
        let flags = [
            (want.race, &self.race),
            (want.countdown, &self.countdown),
            (want.message, &self.message),
            (want.nitrous, &self.nitrous),
            (want.map, &self.map),
        ];
        for (shown, roots) in flags {
            if !shown {
                hidden_roots.extend(roots);
            }
        }
        for index in 0..self.parent.len() {
            let hidden = hidden_roots.iter().any(|root| self.is_under(index, *root));
            rt.set_hidden(ObjectRef { package: self.package, index }, hidden);
        }
    }

    /// `i` is `root` or lies below it.
    fn is_under(&self, mut i: usize, root: usize) -> bool {
        for _ in 0..self.parent.len() {
            if i == root {
                return true;
            }
            let Some(p) = self.parent[i] else { return false };
            i = p;
        }
        false
    }
}

/// The object at the top of the chain of parents of `i`.
fn top_of(parent: &[Option<usize>], mut i: usize) -> usize {
    for _ in 0..parent.len() {
        let Some(p) = parent[i] else { return i };
        i = p;
    }
    i
}

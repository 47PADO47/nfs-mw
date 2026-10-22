use std::collections::HashMap;
use std::sync::Arc;

use crate::package::{ObjectData, ObjectKind, Package, Script, flags};

/// Guid and name lookups, hit on the per-frame message and update paths. They are only looked up, never iterated,
/// so a cheap hasher cannot change results; the fixed seed keeps runs deterministic.
pub type FastMap<K, V> = HashMap<K, V, foldhash::fast::FixedState>;

/// The live state of one object.
#[derive(Clone, Debug)]
pub struct ObjState {
    /// The data block scripts write into.
    pub data: ObjectData,
    /// Private copy of the scripts: move-to tracks rewrite their first key.
    pub scripts: Vec<Script>,
    pub current: Option<usize>,
    /// Clock of the current script, in ticks.
    pub time: i32,
    /// Set when the tracks must be applied even though the clock did not move.
    pub dirty: bool,
    /// Text set by the host (replaces the stored text and the language table).
    pub text: Option<String>,
    pub label: u32,
    /// Texture hash set by the host (replaces the resource).
    pub texture: Option<u32>,
    /// Hidden: set by the host, or at load by the start-hidden definition flag until a script is set on the object.
    pub hidden: bool,
}

/// Definition flag of an object that starts hidden (bit 3 of its flags word). A script set on the object clears it.
pub const START_HIDDEN: u32 = 1 << 3;

/// A package being run.
#[derive(Clone, Debug)]
pub struct Running {
    pub def: Arc<Package>,
    pub objects: Vec<ObjState>,
    pub by_guid: FastMap<u32, usize>,
    pub by_name: FastMap<u32, usize>,
    /// Depth-first order: the order objects are updated in.
    pub order: Arc<[usize]>,
    pub current_button: Option<usize>,
    pub input_enabled: bool,
    /// Objects flagged as buttons, in file order.
    pub buttons: Vec<usize>,
    /// The package takes pad input.
    pub control: bool,
    pub start_equals_accept: bool,
    /// The button each pad bit was pressed on, for the release message.
    pub pressed_on: [Option<usize>; 19],
    /// Counts the changes to anything the tree reads (object data, hidden flags, texts, labels, textures, the language
    /// table). A host that saw the same value last frame can reuse what it built from the tree.
    pub revision: u64,
}

impl Running {
    pub fn new(def: Arc<Package>) -> Self {
        let mut by_guid = FastMap::default();
        let mut by_name = FastMap::default();
        for (i, o) in def.objects.iter().enumerate() {
            by_guid.entry(o.guid).or_insert(i);
            by_name.entry(o.name_hash).or_insert(i);
        }
        let mut children = vec![Vec::new(); def.objects.len()];
        let mut roots = Vec::new();
        for (i, o) in def.objects.iter().enumerate() {
            match o.parent.and_then(|g| by_guid.get(&g).copied()).filter(|p| *p != i) {
                Some(p) if def.objects[p].kind == ObjectKind::Group => children[p].push(i),
                Some(_) | None => roots.push(i),
            }
        }
        let mut order = Vec::with_capacity(def.objects.len());
        let mut stack: Vec<usize> = roots.iter().rev().copied().collect();
        let mut seen = vec![false; def.objects.len()];
        while let Some(i) = stack.pop() {
            if std::mem::replace(&mut seen[i], true) {
                continue;
            }
            order.push(i);
            stack.extend(children[i].iter().rev().copied());
        }
        let objects = def
            .objects
            .iter()
            .map(|o| {
                let init = o.scripts.iter().position(|s| s.id == INIT_SCRIPT);
                ObjState {
                    data: o.data.clone(),
                    scripts: o.scripts.clone(),
                    current: init,
                    time: 0,
                    dirty: true,
                    text: None,
                    label: o.string.as_ref().map_or(0, |s| s.label),
                    texture: None,
                    hidden: o.flags & START_HIDDEN != 0,
                }
            })
            .collect();
        let buttons = def
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.flags & flags::IS_BUTTON != 0 && o.flags & flags::IGNORE_BUTTON == 0)
            .map(|(i, _)| i)
            .collect();
        Self {
            def,
            objects,
            by_guid,
            by_name,
            order: order.into(),
            current_button: None,
            input_enabled: true,
            buttons,
            control: true,
            start_equals_accept: false,
            pressed_on: [None; 19],
            revision: 1,
        }
    }

    /// Records a change to what the tree reads.
    pub fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

/// The script every object starts in.
pub const INIT_SCRIPT: u32 = 0x001744b3;

//! Running packages: object state, script clocks, the message queue, focus, and the host interface.
//! Spec: `docs/specs/feng-runtime.md`.

mod bind;
mod input;
mod interp;
mod messages;
mod nav;
mod state;
mod update;

pub use bind::StringInfo;
pub use input::{PadState, pad};
pub use messages::{Outgoing, PackageCommandKind};
pub use state::{INIT_SCRIPT, ObjState};

use std::collections::VecDeque;
use std::sync::Arc;

use messages::{Message, Target};
use state::Running;

use crate::package::Package;

/// Ticks per second of the FEng clock: 16 ticks per frame at 60 frames per second (inferred, see the spec).
pub const TICKS_PER_SECOND: f32 = 960.0;

/// Identifies a loaded package.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PackageId(pub u32);

/// Identifies an object of a loaded package.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectRef {
    pub package: PackageId,
    /// Index into [`Package::objects`].
    pub index: usize,
}

type StringResolver = Arc<dyn Fn(u32) -> Option<String> + Send + Sync>;

/// All loaded packages and their message queue.
pub struct Runtime {
    packages: Vec<(PackageId, Running)>,
    next_id: u32,
    queue: VecDeque<Message>,
    outgoing: Vec<Outgoing>,
    remainder: f32,
    resolver: Option<StringResolver>,
    pad: PadState,
    pad_next: u32,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            next_id: 1,
            queue: VecDeque::new(),
            outgoing: Vec::new(),
            remainder: 0.0,
            resolver: None,
            pad: PadState::default(),
            pad_next: 0,
        }
    }

    /// Sets the function that turns a label hash into text (the language table). Strings whose label it
    /// knows show that text instead of their own.
    pub fn set_string_resolver(&mut self, f: impl Fn(u32) -> Option<String> + Send + Sync + 'static) {
        self.resolver = Some(Arc::new(f));
    }

    /// Starts running a package and evaluates its `INIT` scripts with zero elapsed ticks, so its first tree
    /// already has the entrance pose. Initialization messages remain queued until the next update.
    pub fn load(&mut self, package: Package) -> PackageId {
        let id = PackageId(self.next_id);
        self.next_id += 1;
        self.packages.push((id, Running::new(Arc::new(package))));
        self.update_package(id, 0);
        id
    }

    pub fn unload(&mut self, id: PackageId) {
        self.packages.retain(|(p, _)| *p != id);
    }

    pub fn ids(&self) -> Vec<PackageId> {
        self.packages.iter().map(|(id, _)| *id).collect()
    }

    pub(crate) fn running(&self, id: PackageId) -> Option<&Running> {
        self.packages.iter().find(|(p, _)| *p == id).map(|(_, r)| r)
    }

    pub(crate) fn running_mut(&mut self, id: PackageId) -> Option<&mut Running> {
        self.packages.iter_mut().find(|(p, _)| *p == id).map(|(_, r)| r)
    }

    /// The parsed package behind an id.
    pub fn package(&self, id: PackageId) -> Option<&Package> {
        self.running(id).map(|r| r.def.as_ref())
    }

    /// The live state of an object.
    pub fn object(&self, o: ObjectRef) -> Option<&ObjState> {
        self.running(o.package)?.objects.get(o.index)
    }

    /// The first object with this name hash (`fe_hash_upper(name)`).
    pub fn find(&self, id: PackageId, name_hash: u32) -> Option<ObjectRef> {
        let index = *self.running(id)?.by_name.get(&name_hash)?;
        Some(ObjectRef { package: id, index })
    }

    pub fn find_guid(&self, id: PackageId, guid: u32) -> Option<ObjectRef> {
        let index = *self.running(id)?.by_guid.get(&guid)?;
        Some(ObjectRef { package: id, index })
    }

    /// Advances every package by `seconds`, then processes the message queue.
    pub fn update(&mut self, seconds: f32) {
        if !seconds.is_finite() || seconds <= 0.0 {
            return;
        }
        let exact = seconds * TICKS_PER_SECOND + self.remainder;
        let ticks = exact.floor();
        self.remainder = exact - ticks;
        let ticks = ticks.min(i32::MAX as f32 / 2.0) as i32;
        self.process_pads(ticks.max(0) as u32);
        for id in self.ids() {
            self.update_package(id, ticks);
        }
        self.process_queue();
    }

    /// Takes the messages and requests addressed to the host since the last call.
    pub fn take_outgoing(&mut self) -> Vec<Outgoing> {
        std::mem::take(&mut self.outgoing)
    }

    /// Makes the object with this GUID the current button, queueing the focus messages.
    pub fn set_focus(&mut self, package: PackageId, guid: u32) {
        let index = self.running(package).and_then(|p| p.by_guid.get(&guid).copied());
        if index.is_some() {
            self.set_focus_index(package, index, true);
        }
    }

    /// The GUID of the current button, if any.
    pub fn focus(&self, package: PackageId) -> Option<u32> {
        let p = self.running(package)?;
        p.current_button.map(|i| p.def.objects[i].guid)
    }

    /// Sends a message to the current button (what a pad press does); its responses run on the next update.
    pub fn send_to_focus(&mut self, package: PackageId, message: u32) {
        let Some(p) = self.running(package) else { return };
        if !p.input_enabled {
            return;
        }
        if let Some(i) = p.current_button {
            self.queue(message, None, package, Target::Object(i));
        }
    }

    pub(crate) fn resolve_label(&self, label: u32) -> Option<String> {
        self.resolver.as_ref().and_then(|f| f(label))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod input_tests;

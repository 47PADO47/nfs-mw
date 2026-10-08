//! Messages: the queue, routing by target, and running response lists.

use super::{ObjectRef, PackageId, Runtime};
use crate::package::{Response, ResponseKind, ResponseParam};

const MSG_LOSE_FOCUS: u32 = 0x55d1e635;
const MSG_GAIN_FOCUS: u32 = 0xabc08912;
/// A script event with this id makes its target the current button.
pub(super) const MSG_SET_BUTTON: u32 = 0x1B3909AA;

/// Where a queued message goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Target {
    /// Every package: its package responses and the objects its `Targ` table lists.
    All,
    Game,
    /// Package responses of every package.
    AllGlobal,
    /// Package responses of the sender package.
    ThisGlobal,
    /// The sender package: responses and `Targ` objects.
    ThisPackage,
    Sound,
    Input,
    Object(usize),
}

impl Target {
    /// From a stored target value: 0 = all, the special values, or the object found for a GUID.
    pub(super) fn from_special(value: u32) -> Option<Self> {
        Some(match value {
            0 => Self::All,
            0xFFFF_FFFF => Self::Game,
            0xFFFF_FFFE => Self::AllGlobal,
            0xFFFF_FFFD => Self::ThisGlobal,
            0xFFFF_FFFC => Self::ThisPackage,
            0xFFFF_FFFB => Self::Sound,
            0xFFFF_FFFA => Self::Input,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Message {
    pub id: u32,
    pub from: Option<usize>,
    pub package: PackageId,
    pub target: Target,
}

/// What a package asks the host to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageCommandKind {
    Switch,
    PushGlobal,
    PushCurrent,
    PushNone,
    Pop,
    RecordMarker,
    SwitchToMarker,
    ClearMarkers,
}

/// Messages and requests that leave the runtime.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outgoing {
    /// A message for the game. `from` is the GUID of the object that sent it.
    Game {
        message: u32,
        package: PackageId,
        from: Option<u32>,
    },
    Sound {
        message: u32,
        package: PackageId,
        from: Option<u32>,
    },
    /// A package-control request; the host owns the package list.
    Package {
        kind: PackageCommandKind,
        name: String,
        from: PackageId,
    },
}

impl Runtime {
    pub(super) fn queue(&mut self, id: u32, from: Option<usize>, package: PackageId, target: Target) {
        self.queue.push_back(Message { id, from, package, target });
    }

    /// Posts a message from the host: to one object, or (`None`) to every package.
    pub fn post(&mut self, package: PackageId, message: u32, target: Option<ObjectRef>) {
        let target = target.map_or(Target::All, |o| Target::Object(o.index));
        self.queue(message, None, package, target);
    }

    /// Runs the queue until it is empty. Responses can queue more messages; a limit stops loops.
    pub(super) fn process_queue(&mut self) {
        let mut budget = 10_000;
        while let Some(m) = self.queue.pop_front() {
            budget -= 1;
            if budget == 0 {
                self.queue.clear();
                break;
            }
            self.route(m);
        }
    }

    fn route(&mut self, m: Message) {
        let from_guid = |rt: &Self| {
            let p = rt.running(m.package)?;
            m.from.and_then(|i| p.def.objects.get(i)).map(|o| o.guid)
        };
        match m.target {
            Target::Game => {
                let from = from_guid(self);
                self.outgoing.push(Outgoing::Game { message: m.id, package: m.package, from });
            }
            Target::Sound => {
                let from = from_guid(self);
                self.outgoing.push(Outgoing::Sound { message: m.id, package: m.package, from });
            }
            // Input control messages are not modelled: the host decides when to feed input.
            Target::Input => {}
            Target::All => {
                let ids: Vec<PackageId> = self.ids();
                for id in ids {
                    self.deliver_global(id, m.id);
                    self.deliver_targets(id, m.id);
                }
            }
            Target::AllGlobal => {
                let ids: Vec<PackageId> = self.ids();
                for id in ids {
                    self.deliver_global(id, m.id);
                }
            }
            Target::ThisGlobal => self.deliver_global(m.package, m.id),
            Target::ThisPackage => {
                self.deliver_global(m.package, m.id);
                self.deliver_targets(m.package, m.id);
            }
            Target::Object(i) => self.deliver_object(m.package, i, m.id),
        }
    }

    fn deliver_global(&mut self, package: PackageId, message: u32) {
        let Some(p) = self.running(package) else { return };
        let Some(list) = p.def.responses_to(message).cloned() else { return };
        self.run_responses(package, None, &list.responses);
    }

    fn deliver_targets(&mut self, package: PackageId, message: u32) {
        let Some(p) = self.running(package) else { return };
        let targets: Vec<usize> = p.def.targets_of(message).iter().filter_map(|g| p.by_guid.get(g).copied()).collect();
        for i in targets {
            self.deliver_object(package, i, message);
        }
    }

    fn deliver_object(&mut self, package: PackageId, index: usize, message: u32) {
        let Some(p) = self.running(package) else { return };
        let Some(def) = p.def.objects.get(index) else { return };
        let Some(list) = def.responses.iter().find(|r| r.message == message).cloned() else { return };
        self.run_responses(package, Some(index), &list.responses);
    }

    /// Runs a response list in order, with if/else/end-if skipping.
    fn run_responses(&mut self, package: PackageId, object: Option<usize>, responses: &[Response]) {
        let mut i = 0;
        while i < responses.len() {
            let r = &responses[i];
            match r.kind {
                ResponseKind::SetScript => {
                    if let Some(o) = object {
                        self.set_script(package, o, r.param.number());
                    }
                }
                ResponseKind::PostToFEng => {
                    let target = match Target::from_special(r.target) {
                        Some(t) => t,
                        None => match self.running(package).and_then(|p| p.by_guid.get(&r.target)) {
                            Some(&idx) => Target::Object(idx),
                            // An unknown GUID becomes a null target, which means "every package".
                            None => Target::All,
                        },
                    };
                    self.queue(r.param.number(), object, package, target);
                }
                ResponseKind::PostToGame => self.queue(r.param.number(), object, package, Target::Game),
                ResponseKind::PostToSound => self.queue(r.param.number(), object, package, Target::Sound),
                ResponseKind::Button(id) => self.button_response(package, id, &r.param),
                ResponseKind::Package(id) => {
                    use super::messages::PackageCommandKind as K;
                    let name = match &r.param {
                        ResponseParam::Text(s) => s.clone(),
                        ResponseParam::Number(_) => String::new(),
                    };
                    let kind = match id {
                        0x200 => Some(K::Switch),
                        0x201 => Some(K::PushGlobal),
                        0x202 => Some(K::PushCurrent),
                        0x203 => Some(K::Pop),
                        0x204 => Some(K::PushNone),
                        0x2C0 => Some(K::RecordMarker),
                        0x2C1 => Some(K::SwitchToMarker),
                        0x2C2 => Some(K::ClearMarkers),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        self.outgoing.push(Outgoing::Package { kind, name, from: package });
                    }
                }
                ResponseKind::IfScriptEquals | ResponseKind::IfScriptNotEquals => {
                    let current = object.and_then(|o| self.current_script_id(package, o));
                    let equal = current == Some(r.param.number());
                    let holds = if r.kind == ResponseKind::IfScriptEquals { equal } else { !equal };
                    if !holds {
                        i = branch_target(responses, i, false);
                    }
                }
                ResponseKind::Else => i = branch_target(responses, i, true),
                ResponseKind::EndIf | ResponseKind::Other(_) => {}
            }
            i += 1;
        }
    }

    fn button_response(&mut self, package: PackageId, id: u32, param: &ResponseParam) {
        match id {
            // Set the active button (GUID, 0 = none).
            0x100 => {
                let guid = param.number();
                let Some(p) = self.running(package) else { return };
                let index = p.by_guid.get(&guid).copied();
                if index.is_none() && guid != 0 {
                    return;
                }
                self.set_focus_index(package, index, index.is_some());
            }
            0x101 => {
                if let Some(p) = self.running_mut(package) {
                    p.input_enabled = param.number() == 1;
                }
            }
            _ => {}
        }
    }

    /// Makes `index` the current button, queueing the lose and gain messages when asked.
    pub(super) fn set_focus_index(&mut self, package: PackageId, index: Option<usize>, send: bool) {
        let Some(old) = self.running(package).map(|p| p.current_button) else { return };
        if send {
            if let Some(o) = old {
                self.queue(MSG_LOSE_FOCUS, None, package, Target::Object(o));
                self.queue(MSG_LOSE_FOCUS, Some(o), package, Target::Sound);
            }
            if let Some(n) = index {
                self.queue(MSG_GAIN_FOCUS, None, package, Target::Object(n));
                self.queue(MSG_GAIN_FOCUS, Some(n), package, Target::Sound);
            }
        }
        if let Some(p) = self.running_mut(package) {
            p.current_button = index;
        }
    }
}

/// The index to continue after when a condition fails (`else_stops` false: the matching else or end-if) or an
/// else branch is reached (`else_stops` true: the matching end-if).
fn branch_target(responses: &[Response], from: usize, reached_else: bool) -> usize {
    let mut depth = 0;
    for (j, r) in responses.iter().enumerate().skip(from + 1) {
        match r.kind {
            ResponseKind::IfScriptEquals | ResponseKind::IfScriptNotEquals => depth += 1,
            ResponseKind::Else if depth == 0 && !reached_else => return j,
            ResponseKind::EndIf => {
                if depth == 0 {
                    return j;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    responses.len()
}

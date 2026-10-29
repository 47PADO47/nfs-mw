//! Opt-in menu telemetry. Observation only: no original-game addresses, code or assets.

#[cfg(test)]
mod tests;
mod writer;

use std::time::{Duration, Instant};

use blackbox_feng::{ObjectRef, Outgoing};
use serde_json::{Value, json};

use super::Screens;

const SAMPLE_PERIOD: Duration = Duration::from_millis(33);
const HEARTBEAT: Duration = Duration::from_secs(1);
const OBJECT_LIMIT: usize = 32;

pub(super) struct Trace {
    writer: writer::Writer,
    frame: u64,
    ui_seconds: f64,
    frame_dt: f32,
    last_input: Option<(u32, u32, bool)>,
    last_device: Option<(crate::input::InputDevice, bool, bool, bool)>,
    last_heartbeat: Instant,
    last_sample: Instant,
    last_state: Value,
}

impl Trace {
    pub fn from_environment() -> Option<Self> {
        let path = std::env::var_os("NFSMW_UI_TRACE")?;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent()?.parent()?;
        let writer = match writer::Writer::create(std::path::Path::new(&path), root) {
            Ok(writer) => writer,
            Err(error) => {
                log::error!("UI trace could not start: {error}");
                return None;
            }
        };
        Some(Self::new(writer))
    }

    fn new(writer: writer::Writer) -> Self {
        let now = Instant::now();
        Self {
            writer,
            frame: 0,
            ui_seconds: 0.0,
            frame_dt: 0.0,
            last_input: None,
            last_device: None,
            last_heartbeat: now,
            last_sample: now.checked_sub(SAMPLE_PERIOD).unwrap_or(now),
            last_state: Value::Null,
        }
    }

    pub fn begin_frame(&mut self, dt: f32, host_pad: u32, menu_pad: u32, await_release: bool) {
        self.frame += 1;
        self.frame_dt = dt;
        if dt.is_finite() && dt > 0.0 {
            self.ui_seconds += f64::from(dt);
        }
        let input = (host_pad, menu_pad, await_release);
        if self.last_input == Some(input) && self.last_heartbeat.elapsed() < HEARTBEAT {
            return;
        }
        self.last_input = Some(input);
        self.last_heartbeat = Instant::now();
        self.emit(
            "input",
            json!({"dt": dt, "host_pad": host_pad, "menu_pad": menu_pad,
            "await_release": await_release}),
        );
    }

    pub fn outgoing(&mut self, outgoing: &[Outgoing]) {
        for out in outgoing {
            match out {
                Outgoing::Game { message, package, from } => {
                    self.emit("game_message", json!({"message": message, "package_id": package.0, "from_guid": from}))
                }
                Outgoing::Sound { message, package, from } => {
                    self.emit("sound_request", json!({"message": message, "package_id": package.0, "from_guid": from}))
                }
                Outgoing::Package { kind, name, from } => self.emit(
                    "package_request",
                    json!({"kind": format!("{kind:?}"), "name": name, "from_package_id": from.0}),
                ),
            }
        }
    }

    fn emit(&mut self, kind: &str, mut data: Value) {
        if let Some(object) = data.as_object_mut() {
            object.insert("ui_seconds".into(), json!(self.ui_seconds));
            object.insert("frame_dt".into(), json!(self.frame_dt));
        }
        self.writer.emit(self.frame, kind, data);
    }
}

impl Screens {
    pub(in crate::frontend) fn trace_device(
        &mut self,
        device: crate::input::InputDevice,
        active_pad: bool,
        disconnected: bool,
        scripted: bool,
    ) {
        let Some(trace) = self.trace.as_mut() else { return };
        let state = (device, active_pad, disconnected, scripted);
        if trace.last_device == Some(state) {
            return;
        }
        trace.last_device = Some(state);
        trace.emit(
            "device",
            json!({"prompt_device": format!("{device:?}"), "active_pad": active_pad,
            "disconnected": disconnected, "scripted": scripted}),
        );
    }

    pub(super) fn trace_stack(&mut self, action: &str) {
        let Some(trace) = self.trace.as_mut() else { return };
        let stack: Vec<Value> =
            self.stack.iter().map(|screen| json!({"package_id": screen.id.0, "name": screen.name})).collect();
        trace.last_state = Value::Null;
        trace.emit("stack", json!({"action": action, "stack": stack}));
    }

    /// A bounded local-state sample after screen logic and stack commands. No text or pictures are copied.
    pub(super) fn trace_sample(&mut self) {
        let Some(trace) = self.trace.as_mut() else { return };
        if trace.last_sample.elapsed() < SAMPLE_PERIOD {
            return;
        }
        trace.last_sample = Instant::now();
        let mut stack = Vec::with_capacity(self.stack.len());
        for screen in &self.stack {
            let focus = self.runtime.current_button(screen.id);
            let list = screen.logic.list(&self.assets);
            stack.push(json!({"package_id": screen.id.0, "name": screen.name,
                "focus_index": focus.map(|object| object.index),
                "logical_selection": list.as_ref().map(|list| list.selected),
                "entry_enabled": list.as_ref().map(|list| list.entries.iter()
                    .map(|entry| entry.enabled).collect::<Vec<_>>())}));
        }
        let mut objects = Vec::new();
        let mut truncated = false;
        if let Some(top) = self.stack.last()
            && let Some(package) = self.runtime.package(top.id)
        {
            let focus = self.runtime.current_button(top.id);
            // Focus first, then top-level package objects. Reading the raw live state avoids building a draw tree.
            let indices = focus.into_iter().map(|object| object.index).chain(
                package
                    .objects
                    .iter()
                    .enumerate()
                    .filter(|(_, object)| object.parent.is_none())
                    .map(|(index, _)| index)
                    .filter(|&index| focus.is_none_or(|focus| index != focus.index)),
            );
            for index in indices {
                if objects.len() == OBJECT_LIMIT {
                    truncated = true;
                    break;
                }
                let object = ObjectRef { package: top.id, index };
                let Some(state) = self.runtime.object(object) else { continue };
                let definition = &package.objects[index];
                objects.push(json!({"index": index, "guid": definition.guid, "name_hash": definition.name_hash,
                    "parent_guid": definition.parent, "script": self.runtime.script_of(object),
                    "script_ticks": state.time, "hidden": state.hidden, "alpha": state.data.alpha(),
                    "local_position": state.data.position().to_array(), "local_size": state.data.size().to_array(),
                    "local_rotation": state.data.rotation().to_array()}));
            }
        }
        let state = json!({"stack": stack, "objects": objects, "object_limit": OBJECT_LIMIT,
            "objects_truncated": truncated});
        if trace.last_state == state {
            return;
        }
        trace.last_state = state.clone();
        trace.emit("state_sample", state);
    }
}

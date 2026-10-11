//! Deterministic system/action state. The host dispatches emitted lists in order.

use std::collections::{BTreeMap, VecDeque};

use crate::{Engine, Package, Response, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueMode {
    Disable,
    Shallow,
    Allow,
    Flush,
    Abort,
}

/// Event-tag hashes supplied by the game; no hash algorithm is imposed here.
#[derive(Debug, Clone, Copy)]
pub struct Tags {
    pub start: u32,
    pub end: u32,
    pub stop: u32,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("unknown event engine {0}")]
    Engine(usize),
    #[error("query/expression filter {0:08X} needs a host evaluator")]
    UnsupportedFilter(u32),
    #[error("event time must be finite and nonnegative")]
    Time,
}

/// One event list to dispatch. A context token is passed only to an immediate
/// action's tagged events; timed events and queued actions use the host's context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Emission {
    pub system: u32,
    pub list: u32,
    pub context: Option<u64>,
}

#[derive(Debug, Clone)]
struct Active {
    action: u32,
    end_state: u32,
    started: f64,
    time: f64,
}

#[derive(Debug, Clone)]
struct Running {
    state: u32,
    active: Option<Active>,
    queue: VecDeque<u32>,
    projected: u32,
}

#[derive(Debug, Clone)]
pub struct Player {
    engine: usize,
    tags: Tags,
    time: f64,
    systems: BTreeMap<u32, Running>,
    /// The original updates in activation order, independent of map/hash order.
    active_order: Vec<u32>,
    emitted: VecDeque<Emission>,
}

impl Player {
    pub fn new(package: &Package, engine: usize, tags: Tags) -> Result<Self, RuntimeError> {
        let definition = package.engines.get(engine).ok_or(RuntimeError::Engine(engine))?;
        let systems = definition
            .systems
            .iter()
            .map(|(&key, system)| {
                let state = valid_state(system, system.initial);
                (key, Running { state, active: None, queue: VecDeque::new(), projected: state })
            })
            .collect();
        Ok(Self { engine, tags, time: 0.0, systems, active_order: Vec::new(), emitted: VecDeque::new() })
    }

    pub fn state(&self, system: u32) -> Option<u32> {
        self.systems.get(&system).map(|s| s.state)
    }

    pub fn busy(&self, system: u32) -> bool {
        self.systems.get(&system).is_some_and(|s| s.active.is_some())
    }

    pub fn pop(&mut self) -> Option<Emission> {
        self.emitted.pop_front()
    }

    /// Deliver to one system (`Some`) or every system (`None`), in stored order.
    pub fn stimulate(
        &mut self,
        package: &Package,
        target: Option<u32>,
        stimulus: u32,
        mode: QueueMode,
        context: Option<u64>,
    ) -> Result<bool, RuntimeError> {
        let engine = package.engines.get(self.engine).ok_or(RuntimeError::Engine(self.engine))?;
        if let Some(target) = target {
            return self.send(package, engine, target, stimulus, mode, context);
        }
        let mut handled = false;
        for &key in engine.systems.keys() {
            handled |= self.send(package, engine, key, stimulus, mode, context)?;
        }
        Ok(handled)
    }

    fn send(
        &mut self,
        package: &Package,
        engine: &Engine,
        key: u32,
        stimulus: u32,
        mode: QueueMode,
        context: Option<u64>,
    ) -> Result<bool, RuntimeError> {
        let Some(running) = self.systems.get_mut(&key) else { return Ok(false) };
        if mode == QueueMode::Flush {
            running.queue.clear();
            running.projected = running.active.as_ref().map_or(running.state, |a| a.end_state);
        }
        if mode == QueueMode::Abort {
            self.finish(package, engine, key, self.tags.stop, true, context)?;
        }
        let running = &mut self.systems.get_mut(&key).unwrap();
        let definition = &engine.systems[&key];
        if running.active.is_none() {
            let Some(response) = response(definition, running.state, stimulus)? else { return Ok(false) };
            self.start(package, engine, key, response, context)?;
            return Ok(true);
        }
        let capacity = match mode {
            QueueMode::Allow => 4,
            QueueMode::Shallow => 1,
            _ => 0,
        };
        if running.queue.len() >= capacity {
            return Ok(false);
        }
        let Some(response) = response(definition, running.projected, stimulus)? else { return Ok(false) };
        running.queue.push_back(stimulus);
        running.projected = response.end_state;
        Ok(true)
    }

    fn start(
        &mut self,
        package: &Package,
        engine: &Engine,
        key: u32,
        response: Response,
        context: Option<u64>,
    ) -> Result<(), RuntimeError> {
        let running = self.systems.get_mut(&key).unwrap();
        running.active =
            Some(Active { action: response.action, end_state: response.end_state, started: self.time, time: 0.0 });
        if running.queue.is_empty() {
            running.projected = response.end_state;
        }
        self.active_order.push(key);
        self.tag(package, key, self.tags.start, context);
        self.timed(package, key, -1.0, 0.0);
        let duration = package.actions[&response.action].duration;
        if duration == 0.0 {
            self.finish(package, engine, key, self.tags.end, false, None)?;
        }
        Ok(())
    }

    /// Advance once per simulation tick, never during presentation/interpolation.
    pub fn advance(&mut self, package: &Package, dt: f64) -> Result<(), RuntimeError> {
        if !dt.is_finite() || dt < 0.0 || !(self.time + dt).is_finite() {
            return Err(RuntimeError::Time);
        }
        if dt == 0.0 {
            return Ok(());
        }
        self.time += dt;
        let engine = package.engines.get(self.engine).ok_or(RuntimeError::Engine(self.engine))?;
        for key in self.active_order.clone() {
            let Some(active) = self.systems.get_mut(&key).and_then(|s| s.active.as_mut()) else { continue };
            let before = active.time;
            active.time = self.time - active.started;
            let after = active.time;
            let duration = f64::from(package.actions[&active.action].duration);
            self.timed(package, key, before, after);
            if duration >= 0.0 && after >= duration {
                self.finish(package, engine, key, self.tags.end, false, None)?;
            }
        }
        Ok(())
    }

    fn tag(&mut self, package: &Package, key: u32, tag: u32, context: Option<u64>) {
        let Some(active) = self.systems[&key].active.as_ref() else { return };
        if let Some(&list) = package.actions[&active.action].tags.get(&tag) {
            self.emitted.push_back(Emission { system: key, list, context });
        }
    }

    fn timed(&mut self, package: &Package, key: u32, before: f64, after: f64) {
        let Some(active) = self.systems[&key].active.as_ref() else { return };
        for &(time, list) in &package.actions[&active.action].times {
            if f64::from(time) > before && f64::from(time) <= after {
                self.emitted.push_back(Emission { system: key, list, context: None });
            }
        }
    }

    fn finish(
        &mut self,
        package: &Package,
        engine: &Engine,
        key: u32,
        tag: u32,
        flush: bool,
        context: Option<u64>,
    ) -> Result<(), RuntimeError> {
        self.tag(package, key, tag, context);
        let running = self.systems.get_mut(&key).unwrap();
        if flush {
            running.queue.clear();
        }
        if let Some(active) = running.active.take() {
            running.state = valid_state(&engine.systems[&key], active.end_state);
        }
        self.active_order.retain(|&id| id != key);
        let next = running.queue.pop_front();
        if running.queue.is_empty() {
            running.projected = running.state;
        }
        let Some(stimulus) = next else { return Ok(()) };
        let Some(response) = response(&engine.systems[&key], running.state, stimulus)? else { return Ok(()) };
        self.start(package, engine, key, response, context)
    }
}

fn valid_state(system: &crate::System, state: u32) -> u32 {
    if system.states.contains_key(&state) {
        return state;
    }
    system.default
}

fn response(system: &crate::System, state: u32, stimulus: u32) -> Result<Option<Response>, RuntimeError> {
    let State { filter, responses } = &system.states[&valid_state(system, state)];
    if let Some(filter) = filter {
        return Err(RuntimeError::UnsupportedFilter(*filter));
    }
    Ok(responses.get(&stimulus).copied())
}

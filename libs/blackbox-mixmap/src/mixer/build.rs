//! Building a [`Mixer`]: one copy of every element per instance of its state, with every id resolved to the
//! element or input it reads. Spec: `docs/specs/dynamic-mixer.md` §3.

use std::collections::HashMap;

use super::{Block, ControlNode, EventNode, Graph, InputKey, InputSource, Mixer, ObjectRef, SpatialNode, Src, SubNode};
use crate::id::{SourceId, SourceKind};
use crate::map::{EnvelopeKind, MixMap, State};
use crate::shape::{UNITY, q15_from_db};

/// Most copies of a state (the original's limit: the instance is a 5-bit field).
pub const MAX_INSTANCES: u8 = 32;

/// One frame of the original, in milliseconds, for the event times.
const FRAME_MS: f32 = 16.66667;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Cat {
    Control,
    Sub,
    Master,
    Spatial,
    Event,
}

struct Builder<'a> {
    map: &'a MixMap,
    refs: Vec<u8>,
    lookup: HashMap<(Cat, u8, u8, u8), usize>,
    input_index: HashMap<InputKey, usize>,
    block_index: HashMap<ObjectRef, usize>,
    blocks: Vec<Block>,
}

pub(super) fn build(map: &MixMap, instances: &[u8]) -> Mixer {
    let refs: Vec<u8> = (0..map.states.len())
        .map(|s| {
            let wanted = instances.get(s).copied().unwrap_or(0).min(MAX_INSTANCES);
            if map.state(s).is_some() { wanted } else { 0 }
        })
        .collect();
    let mut b = Builder {
        map,
        refs,
        lookup: HashMap::new(),
        input_index: HashMap::new(),
        block_index: HashMap::new(),
        blocks: Vec::new(),
    };
    b.register();
    b.create()
}

impl<'a> Builder<'a> {
    /// The instances of every state that has any, as `(state number, instance, state)`.
    fn instances(&self) -> Vec<(u8, u8, &'a State)> {
        let mut out = Vec::new();
        for (s, state) in self.map.states.iter().enumerate() {
            let Some(state) = state else { continue };
            for inst in 0..self.refs[s] {
                out.push((s as u8, inst, state));
            }
        }
        out
    }

    /// First pass: give every element its global index, so ids can point forward.
    fn register(&mut self) {
        let mut next = HashMap::new();
        for (s, inst, state) in self.instances() {
            let counts = [
                (Cat::Control, state.controls.len()),
                (Cat::Sub, state.subs.len()),
                (Cat::Master, state.masters.len()),
                (Cat::Spatial, state.spatial.len()),
                (Cat::Event, state.events.len()),
            ];
            for (cat, n) in counts {
                let counter: &mut usize = next.entry(cat).or_insert(0);
                for k in 0..n {
                    self.lookup.insert((cat, s, inst, k as u8), *counter);
                    *counter += 1;
                }
            }
        }
    }

    fn input(&mut self, key: InputKey) -> Src {
        let next = self.input_index.len();
        Src::Input(*self.input_index.entry(key).or_insert(next))
    }

    /// What `id` (with its instance bits set) reads; `level` asks for the dB value of controls, events and 3D controls.
    fn resolve(&mut self, id: SourceId, level: bool) -> Src {
        let (state, inst) = (id.state(), id.instance());
        let find = |b: &Self, cat: Cat| b.lookup.get(&(cat, state, inst, id.element())).copied();
        match id.kind() {
            SourceKind::Control => match find(self, Cat::Control) {
                Some(i) if level => Src::CtlLevel(i),
                Some(i) => Src::CtlCurve(i),
                None => Src::Zero,
            },
            SourceKind::Channel => {
                let cat = if id.is_sub_channel() { Cat::Sub } else { Cat::Master };
                match (find(self, cat), id.is_sub_channel()) {
                    (Some(i), true) => Src::Sub(i),
                    (Some(i), false) => Src::Master(i),
                    (None, _) => Src::Zero,
                }
            }
            SourceKind::Object | SourceKind::Controller => {
                let source =
                    if id.kind() == SourceKind::Object { InputSource::Object } else { InputSource::Controller };
                self.input(InputKey { source, state, instance: inst, object: id.object(), index: id.input_index() })
            }
            SourceKind::Spatial => match find(self, Cat::Spatial) {
                Some(i) if level => Src::SpatialDb(i),
                Some(i) => Src::SpatialQ(i),
                None => Src::Zero,
            },
            SourceKind::Event => match find(self, Cat::Event) {
                Some(i) if level => Src::EventDb(i),
                Some(i) => Src::EventQ(i),
                None => Src::Zero,
            },
            SourceKind::Invalid => Src::Zero,
        }
    }

    /// The ids an input list stands for: an id in the element's own state means its own instance, one in
    /// another state all the instances of that state.
    fn expand(&self, id: SourceId, own_state: u8, own_instance: u8) -> Vec<SourceId> {
        let state = id.state();
        if state == own_state {
            return vec![id.with_instance(own_instance)];
        }
        let copies = self.refs.get(usize::from(state)).copied().unwrap_or(0);
        (0..copies).map(|m| id.with_instance(m)).collect()
    }

    fn resolve_list(&mut self, ids: &[SourceId], own_state: u8, inst: u8, level: bool) -> Vec<Src> {
        let mut out = Vec::new();
        for &id in ids {
            for expanded in self.expand(id, own_state, inst) {
                out.push(self.resolve(expanded, level));
            }
        }
        out
    }

    fn block(&mut self, object: ObjectRef) -> usize {
        if let Some(&i) = self.block_index.get(&object) {
            return i;
        }
        self.blocks.push(Block { attached: false, slots: [0; 32] });
        self.block_index.insert(object, self.blocks.len() - 1);
        self.blocks.len() - 1
    }

    /// Second pass: the nodes, in the order of the lookup.
    fn create(mut self) -> Mixer {
        let mut m = Mixer {
            graph: Graph::default(),
            controls: Vec::new(),
            events: Vec::new(),
            spatial: Vec::new(),
            subs: Vec::new(),
            masters: Vec::new(),
            blocks: Vec::new(),
            input_index: HashMap::new(),
            block_index: HashMap::new(),
            camera: 0,
        };
        for (s, inst, state) in self.instances() {
            for c in &state.controls {
                let (offset, depth) = c.offset_and_depth();
                let input = self.resolve(c.input.with_instance(inst), false);
                let scales = self.resolve_list(&c.scales, c.input.state(), inst, false);
                m.controls.push(ControlNode {
                    shape: c.shape(),
                    input,
                    offset,
                    ratio: UNITY - q15_from_db(depth),
                    scales,
                });
            }
            for e in &state.events {
                m.events.push(self.event_node(e, inst));
            }
            for sp in &state.spatial {
                m.spatial.push(self.spatial_node(sp, inst));
            }
            for sub in &state.subs {
                let inputs = self.resolve_list(&sub.inputs, s, inst, true);
                m.subs.push(SubNode { upper: sub.upper, lower: sub.lower, inputs });
            }
            for mc in &state.masters {
                m.masters.push(self.master_node(mc, s, inst));
            }
        }
        m.graph = Graph {
            inputs: vec![0; self.input_index.len()],
            ctl_curve: vec![UNITY; m.controls.len()],
            ctl_level: vec![0; m.controls.len()],
            sub: vec![0; m.subs.len()],
            master: vec![0; m.masters.len()],
            spatial_q: vec![UNITY; m.spatial.len()],
            spatial_db: vec![0; m.spatial.len()],
            spatial_azimuth: vec![0; m.spatial.len()],
            event_q: vec![UNITY; m.events.len()],
            event_db: vec![0; m.events.len()],
        };
        m.input_index = self.input_index;
        m.block_index = self.block_index;
        m.blocks = self.blocks;
        m
    }

    fn event_node(&mut self, e: &crate::map::Event, inst: u8) -> EventNode {
        let state = e.id.state();
        let kind = e.kind();
        // A zero time counts as one frame where the envelope uses it.
        let uses = match kind {
            EnvelopeKind::Ar | EnvelopeKind::Atr => [true, false, true],
            EnvelopeKind::Asr => [true, true, true],
            EnvelopeKind::Lfo => [false, false, false],
        };
        let time = |i: usize| {
            let frames = (e.params[i] & 0xFFF).max(u32::from(uses[i]));
            frames as f32 * FRAME_MS
        };
        EventNode {
            kind,
            trigger: self.resolve(e.trigger.with_instance(inst), false),
            swing: e.swing,
            times: [time(0), time(1), time(2)],
            shapes: [((e.params[0] >> 12) & 0xF) as u8, ((e.params[2] >> 12) & 0xF) as u8],
            scales: self.resolve_list(&e.scales, state, inst, false),
            elapsed: 0.0,
            reset_time: 0.0,
            reset: false,
            reset_level: -10_000,
        }
    }

    fn spatial_node(&mut self, sp: &crate::map::Spatial, inst: u8) -> SpatialNode {
        let (state, object) = (sp.id.state(), sp.id.object());
        let mut block = [Src::Zero; 5];
        for (slot, index) in [0u8, 1, 2, 3, 15].into_iter().enumerate() {
            block[slot] = self.input(InputKey::controller(state, inst, object, index));
        }
        SpatialNode { records: sp.records.clone(), current: 0, block }
    }

    fn master_node(&mut self, mc: &crate::map::MasterChannel, state: u8, inst: u8) -> super::MasterNode {
        let object = ObjectRef { state: mc.object.state(), instance: inst, object: mc.object.object() };
        let block = self.block(object);
        let mut spatial = Vec::new();
        let mut plain = Vec::new();
        for &id in &mc.inputs {
            if id.kind() != SourceKind::Spatial {
                plain.push(id);
                continue;
            }
            if let Src::SpatialQ(i) = self.resolve(id.with_instance(inst), false) {
                spatial.push(i);
            }
        }
        super::MasterNode {
            block,
            base: mc.base,
            inputs: self.resolve_list(&plain, state, inst, true),
            spatial,
            kind: mc.kind,
            words: mc.words.clone(),
        }
    }
}

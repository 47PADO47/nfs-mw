//! The evaluator: a [`Mixer`] instantiates a [`MixMap`] for the sound objects of a game and turns the values the
//! game publishes into the levels, pitches and filters the objects read. Spec: `docs/specs/dynamic-mixer.md`.

mod build;
mod event;
mod frame;
mod output;
mod spatial;

use std::collections::HashMap;
use std::sync::Arc;

use crate::map::{MixMap, OutputKind, PresetWord};
use crate::shape::pitch_ratio;

pub use self::build::MAX_INSTANCES;

/// Where an element reads a value from, resolved once when the mixer is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Src {
    /// Nothing there: reads 0.
    Zero,
    /// A value the game publishes.
    Input(usize),
    /// A control's curve output (Q15).
    CtlCurve(usize),
    /// A control's level (hundredths of a dB).
    CtlLevel(usize),
    Sub(usize),
    Master(usize),
    SpatialQ(usize),
    SpatialDb(usize),
    EventQ(usize),
    EventDb(usize),
}

/// The value of every element, by kind.
#[derive(Debug, Default)]
pub(crate) struct Graph {
    pub inputs: Vec<i32>,
    pub ctl_curve: Vec<i32>,
    pub ctl_level: Vec<i32>,
    pub sub: Vec<i32>,
    pub master: Vec<i32>,
    pub spatial_q: Vec<i32>,
    pub spatial_db: Vec<i32>,
    pub spatial_azimuth: Vec<i32>,
    pub event_q: Vec<i32>,
    pub event_db: Vec<i32>,
}

impl Graph {
    pub fn read(&self, src: Src) -> i32 {
        match src {
            Src::Zero => 0,
            Src::Input(i) => self.inputs[i],
            Src::CtlCurve(i) => self.ctl_curve[i],
            Src::CtlLevel(i) => self.ctl_level[i],
            Src::Sub(i) => self.sub[i],
            Src::Master(i) => self.master[i],
            Src::SpatialQ(i) => self.spatial_q[i],
            Src::SpatialDb(i) => self.spatial_db[i],
            Src::EventQ(i) => self.event_q[i],
            Src::EventDb(i) => self.event_db[i],
        }
    }
}

/// Whether an input belongs to a sound object or to a sound controller (the two kinds of ids 2 and 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputSource {
    Object,
    Controller,
}

/// The address of one value the game publishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputKey {
    pub source: InputSource,
    pub state: u8,
    pub instance: u8,
    /// The object or controller number.
    pub object: u8,
    /// 0 to 15.
    pub index: u8,
}

impl InputKey {
    pub fn object(state: u8, instance: u8, object: u8, index: u8) -> Self {
        Self { source: InputSource::Object, state, instance, object, index }
    }

    pub fn controller(state: u8, instance: u8, object: u8, index: u8) -> Self {
        Self { source: InputSource::Controller, state, instance, object, index }
    }
}

/// A sound object whose outputs the game reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectRef {
    pub state: u8,
    pub instance: u8,
    pub object: u8,
}

/// The output slots of one object, and whether the object exists in the game.
#[derive(Debug, Clone)]
pub(crate) struct Block {
    pub attached: bool,
    pub slots: [u16; 32],
}

#[derive(Debug)]
pub(crate) struct ControlNode {
    pub shape: u8,
    pub input: Src,
    pub offset: i32,
    /// `0x7FFF - q15(depth)`.
    pub ratio: i32,
    pub scales: Vec<Src>,
}

#[derive(Debug)]
pub(crate) struct EventNode {
    pub kind: crate::map::EnvelopeKind,
    pub trigger: Src,
    pub swing: i32,
    /// Attack, sustain and release times (ms) and the attack and release shapes.
    pub times: [f32; 3],
    pub shapes: [u8; 2],
    pub scales: Vec<Src>,
    pub elapsed: f32,
    pub reset_time: f32,
    pub reset: bool,
    pub reset_level: i32,
}

#[derive(Debug)]
pub(crate) struct SpatialNode {
    pub records: Vec<crate::map::SpatialRecord>,
    pub current: usize,
    /// The input block of the controller: indices 0, 1, 2, 3 and 15.
    pub block: [Src; 5],
}

#[derive(Debug)]
pub(crate) struct SubNode {
    pub upper: i32,
    pub lower: i32,
    pub inputs: Vec<Src>,
}

#[derive(Debug)]
pub(crate) struct MasterNode {
    pub block: usize,
    pub base: i32,
    pub inputs: Vec<Src>,
    /// Indices of the spatial controls among its inputs, in input order.
    pub spatial: Vec<usize>,
    pub kind: OutputKind,
    pub words: Vec<PresetWord>,
}

/// An instantiated mixer map.
#[derive(Debug)]
pub struct Mixer {
    pub(crate) graph: Graph,
    pub(crate) controls: Vec<ControlNode>,
    pub(crate) events: Vec<EventNode>,
    pub(crate) spatial: Vec<SpatialNode>,
    pub(crate) subs: Vec<SubNode>,
    pub(crate) masters: Vec<MasterNode>,
    pub(crate) blocks: Vec<Block>,
    input_index: HashMap<InputKey, usize>,
    block_index: HashMap<ObjectRef, usize>,
    camera: u8,
}

impl Mixer {
    /// Instantiates `map`: `instances[s]` copies of state `s` (missing entries and 0 mean none, at most
    /// [`MAX_INSTANCES`]).
    pub fn new(map: Arc<MixMap>, instances: &[u8]) -> Mixer {
        build::build(&map, instances)
    }

    /// Publishes a value. Keys nothing in the map reads are ignored.
    pub fn set_input(&mut self, key: InputKey, value: i32) {
        if let Some(&i) = self.input_index.get(&key) {
            self.graph.inputs[i] = value;
        }
    }

    /// The value last published at `key` (0 when never set or unknown).
    pub fn input(&self, key: InputKey) -> i32 {
        self.input_index.get(&key).map_or(0, |&i| self.graph.inputs[i])
    }

    /// Says whether a sound object exists in the game. Master channels of an object that is not attached
    /// are silent.
    pub fn attach(&mut self, object: ObjectRef, attached: bool) {
        if let Some(&b) = self.block_index.get(&object) {
            self.blocks[b].attached = attached;
        }
    }

    /// The camera state (0 default, 1 bumper, 2 in car, 3 jump, 4 cut scene, 5 collision) that 3D controls pick
    /// their rolloff for.
    pub fn set_camera(&mut self, camera: u8) {
        if camera == self.camera {
            return;
        }
        self.camera = camera;
        spatial::select_camera(&mut self.spatial, camera);
    }

    /// Advances by `dt` seconds and recomputes every output.
    pub fn process(&mut self, dt: f32) {
        frame::process(self, dt);
    }

    /// The 16-bit word of an output slot; `None` when the map has no channel for the object.
    pub fn raw_slot(&self, object: ObjectRef, slot: usize) -> Option<u16> {
        let block = &self.blocks[*self.block_index.get(&object)?];
        block.slots.get(slot).copied()
    }

    /// A volume slot as a gain, 0 to 1.
    pub fn volume(&self, object: ObjectRef, slot: usize) -> Option<f32> {
        Some(f32::from(self.raw_slot(object, slot)? & 0x7FFF) / 32767.0)
    }

    /// A pitch slot as a playback ratio (1 = unchanged), quantised like the original's 4096 scale.
    pub fn pitch(&self, object: ObjectRef, slot: usize) -> Option<f32> {
        let cents = i32::from(self.raw_slot(object, slot)? as i16);
        Some(((pitch_ratio(cents) * 4096.0) as i32) as f32 / 4096.0)
    }

    /// A filter slot: the cut-off, 25000 when fully open.
    pub fn filter(&self, object: ObjectRef, slot: usize) -> Option<f32> {
        Some(f32::from(self.raw_slot(object, slot)? & 0x7FFF))
    }

    /// An azimuth slot (a 16-bit angle, 0 straight ahead).
    pub fn azimuth(&self, object: ObjectRef, slot: usize) -> Option<u16> {
        self.raw_slot(object, slot)
    }

    /// How many elements the mixer holds: controls, events, 3D controls, sub-mix and master channels.
    pub fn element_counts(&self) -> [usize; 5] {
        [self.controls.len(), self.events.len(), self.spatial.len(), self.subs.len(), self.masters.len()]
    }
}

//! Tests on synthetic maps written with [`MapBuilder`](crate::MapBuilder).

mod eval;
mod events;
mod parse;
mod spatial;

use std::sync::Arc;

use crate::{
    Control, MapBuilder, MasterChannel, MixMap, Mixer, ObjectRef, OutputKind, PresetWord, SourceId, State, SubChannel,
};

/// A controller input id: curve `shape`, state, controller number, input index.
pub(crate) fn controller(shape: u32, state: u32, object: u32, index: u32) -> SourceId {
    SourceId(0x6000_0000 | (shape << 24) | (state << 16) | (object << 4) | index)
}

/// An object input id.
pub(crate) fn object_input(state: u32, object: u32, index: u32) -> SourceId {
    SourceId(0x4000_0000 | (state << 16) | (object << 4) | index)
}

/// The level of control `n` of `state`.
pub(crate) fn control_ref(state: u32, n: u32) -> SourceId {
    SourceId((state << 16) | n)
}

pub(crate) fn sub_ref(state: u32, n: u32) -> SourceId {
    SourceId(0x3000_0000 | (state << 16) | n)
}

pub(crate) fn event_ref(state: u32, n: u32) -> SourceId {
    SourceId(0xA000_0000 | (state << 16) | n)
}

pub(crate) fn spatial_ref(state: u32, n: u32) -> SourceId {
    SourceId(0x8000_0000 | (state << 16) | n)
}

/// A control with a swing word that carries its scale count.
pub(crate) fn control(input: SourceId, swing: u32, scales: Vec<SourceId>) -> Control {
    Control { input, swing: (swing & 0xFFFF) | ((scales.len() as u32) << 16), scales }
}

pub(crate) fn word(slot: u8) -> PresetWord {
    PresetWord { slot, spatial: 0, azimuth: false, offset: 0 }
}

pub(crate) fn master(object: u32, kind: OutputKind, inputs: Vec<SourceId>, words: Vec<PresetWord>) -> MasterChannel {
    MasterChannel { object: SourceId(0x4000_0000 | (object << 4)), base: 0, inputs, kind, words }
}

pub(crate) fn sub(inputs: Vec<SourceId>, lower: i32, upper: i32) -> SubChannel {
    SubChannel { upper, lower, inputs }
}

pub(crate) fn obj(state: u8, instance: u8, object: u8) -> ObjectRef {
    ObjectRef { state, instance, object }
}

/// A mixer over the given states (state number, state, instances), all objects attached.
pub(crate) fn mixer(states: Vec<(usize, State)>, instances: &[u8]) -> Mixer {
    let mut builder = MapBuilder::new(states.iter().map(|s| s.0 + 1).max().unwrap_or(1));
    for (n, state) in states {
        builder = builder.state(n, state);
    }
    let map = Arc::new(MixMap::parse(&builder.build()).expect("map parses"));
    let mut mixer = Mixer::new(map, instances);
    for (state, &copies) in instances.iter().enumerate() {
        for instance in 0..copies {
            for object in 0..8 {
                mixer.attach(obj(state as u8, instance, object), true);
            }
        }
    }
    mixer
}

/// Runs `frames` frames of 1/60 s.
pub(crate) fn run(mixer: &mut Mixer, frames: usize) {
    for _ in 0..frames {
        mixer.process(1.0 / 60.0);
    }
}

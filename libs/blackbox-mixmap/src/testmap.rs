//! Writing maps: the inverse of [`MixMap::parse`], for tests and examples that need a small map without a game
//! install.

use crate::map::{MasterChannel, OutputKind, State};

fn put(out: &mut Vec<u8>, word: u32) {
    out.extend_from_slice(&word.to_le_bytes());
}

fn kind_nibble(kind: OutputKind) -> u32 {
    match kind {
        OutputKind::Volume => 0,
        OutputKind::Pitch => 1,
        OutputKind::Filter => 2,
        OutputKind::Depth => 4,
        OutputKind::Other => 3,
    }
}

fn section_header(out: &mut Vec<u8>, count: usize) {
    put(out, count as u32);
    for _ in 0..3 {
        put(out, 0);
    }
}

fn controls(out: &mut Vec<u8>, state: &State) {
    section_header(out, state.controls.len());
    for c in &state.controls {
        put(out, c.input.0);
        put(out, (c.swing & 0xFFFF) | ((c.scales.len() as u32) << 16));
        c.scales.iter().for_each(|s| put(out, s.0));
    }
}

fn events(out: &mut Vec<u8>, state: &State) {
    section_header(out, state.events.len());
    for e in &state.events {
        put(out, e.id.0);
        put(out, (e.swing as u32 & 0xFFFF) | ((e.scales.len() as u32) << 16));
        put(out, e.trigger.0);
        e.params.iter().for_each(|&p| put(out, p));
        e.scales.iter().for_each(|s| put(out, s.0));
    }
}

fn spatial(out: &mut Vec<u8>, state: &State) {
    section_header(out, state.spatial.len());
    for s in &state.spatial {
        put(out, (s.id.0 & 0xF0FF_FFFF) | ((s.records.len() as u32) << 24));
        for r in &s.records {
            put(out, r.info);
            put(out, r.curves);
            r.ranges.iter().for_each(|&q| put(out, q));
        }
    }
}

fn subs(out: &mut Vec<u8>, state: &State, number: u32) {
    section_header(out, state.subs.len());
    for (k, s) in state.subs.iter().enumerate() {
        put(out, 0xD000_0000 | ((s.inputs.len() as u32) << 16) | (number << 8) | k as u32);
        put(out, ((s.upper as u32 & 0x7FFF) << 16) | (s.lower as u32 & 0xFFFF));
        s.inputs.iter().for_each(|i| put(out, i.0));
    }
}

fn masters(out: &mut Vec<u8>, masters: &[MasterChannel], number: u32) {
    section_header(out, masters.len());
    for (k, m) in masters.iter().enumerate() {
        put(out, 0xC000_0000 | ((m.inputs.len() as u32) << 16) | (number << 8) | k as u32);
        put(out, ((m.base as u32 & 0xFFFF) << 16) | 0xD8F0);
        put(out, m.object.0);
        m.inputs.iter().for_each(|i| put(out, i.0));
    }
}

fn presets(out: &mut Vec<u8>, masters: &[MasterChannel]) {
    for m in masters {
        put(out, (kind_nibble(m.kind) << 24) | m.words.len() as u32);
        for w in &m.words {
            let azimuth = u32::from(w.azimuth) << 31;
            put(out, azimuth | (u32::from(w.slot) << 26) | (u32::from(w.spatial) << 21) | (w.offset as u32 & 0xFFFF));
        }
    }
}

/// The bytes of a state block.
fn state_block(state: &State, number: u32) -> Vec<u8> {
    let mut body = Vec::new();
    let mut offsets = [-1i32; 6];
    // An empty section is left out (offset -1), as the shipped maps do.
    let mut section = |index: usize, present: bool, write: &dyn Fn(&mut Vec<u8>)| {
        if !present {
            return;
        }
        offsets[index] = 32 + body.len() as i32;
        write(&mut body);
    };
    section(0, !state.controls.is_empty(), &|b| controls(b, state));
    section(5, !state.events.is_empty(), &|b| events(b, state));
    section(1, !state.spatial.is_empty(), &|b| spatial(b, state));
    section(2, !state.subs.is_empty(), &|b| subs(b, state, number));
    section(3, !state.masters.is_empty(), &|b| masters(b, &state.masters, number));
    section(4, !state.masters.is_empty(), &|b| presets(b, &state.masters));
    let mut out = Vec::new();
    put(&mut out, 0x001F_0000 | number);
    offsets.iter().for_each(|&o| put(&mut out, o as u32));
    put(&mut out, u32::MAX);
    out.extend(body);
    out
}

/// Builds a map file from parsed pieces.
#[derive(Debug, Clone, Default)]
pub struct MapBuilder {
    states: Vec<Option<State>>,
}

impl MapBuilder {
    /// A map with `count` states, none present yet.
    pub fn new(count: usize) -> Self {
        Self { states: vec![None; count] }
    }

    /// Puts `state` at number `number`.
    pub fn state(mut self, number: usize, state: State) -> Self {
        self.states[number] = Some(state);
        self
    }

    /// The bytes of the file.
    pub fn build(&self) -> Vec<u8> {
        let count = self.states.len();
        let blocks: Vec<Option<Vec<u8>>> =
            self.states.iter().enumerate().map(|(n, s)| s.as_ref().map(|s| state_block(s, n as u32))).collect();
        let mut out = Vec::new();
        for word in [0, count as u32, 16, u32::MAX] {
            put(&mut out, word);
        }
        let mut at = 16 + 4 * count;
        for block in &blocks {
            match block {
                Some(b) => {
                    put(&mut out, at as u32);
                    at += b.len();
                }
                None => put(&mut out, u32::MAX),
            }
        }
        blocks.into_iter().flatten().for_each(|b| out.extend(b));
        out
    }
}

/// A map of one state with one master channel for object 1: a control reads input 0 of controller 0 and the
/// channel adds it; the volume goes to slot 2. Enough for the example in the crate docs.
pub fn minimal_map() -> Vec<u8> {
    use crate::id::SourceId;
    use crate::map::{Control, PresetWord};
    let state = State {
        controls: vec![Control { input: SourceId(0x6800_0000), swing: 0xD8F0, scales: vec![] }],
        masters: vec![MasterChannel {
            object: SourceId(0x4000_0010),
            base: 0,
            inputs: vec![SourceId(0x0000_0000)],
            kind: OutputKind::Volume,
            words: vec![PresetWord { slot: 2, spatial: 0, azimuth: false, offset: 0 }],
        }],
        ..State::default()
    };
    MapBuilder::new(1).state(0, state).build()
}

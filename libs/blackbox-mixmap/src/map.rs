//! A parsed mixer map. Layout: `docs/formats/mixmap.md`.

use crate::error::{Error, Result};
use crate::id::SourceId;

/// A control: a curve of one input, scaled by others, giving a level in hundredths of a dB (or cents).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Control {
    /// The input; its bits 24 to 27 are the curve shape.
    pub input: SourceId,
    /// The raw swing word (`bit 15` clear: positive offset in the low 15 bits; set: signed depth in the low 16).
    pub swing: u32,
    pub scales: Vec<SourceId>,
}

impl Control {
    pub fn shape(&self) -> u8 {
        self.input.nibble()
    }

    /// `(offset, depth)`: what the control adds at full input and the cut at zero input, both signed.
    pub fn offset_and_depth(&self) -> (i32, i32) {
        if self.swing & 0x8000 == 0 {
            let offset = (self.swing & 0x7FFF) as i32;
            return (offset, -offset);
        }
        (0, i32::from(self.swing as u16 as i16))
    }
}

/// The kind of envelope of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeKind {
    /// Attack then release, timed.
    Ar,
    /// Attack, sustain, release, timed.
    Asr,
    /// Attack, held while the trigger is on, release when it drops.
    Atr,
    /// Not implemented by the original.
    Lfo,
}

/// An envelope that a trigger starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: SourceId,
    /// The level, signed; negative fades down.
    pub swing: i32,
    pub trigger: SourceId,
    /// Attack, sustain and release words: time in frames of 1/60 s in bits 0 to 11, curve shape in 12 to 15.
    pub params: [u32; 3],
    pub scales: Vec<SourceId>,
}

impl Event {
    pub fn kind(&self) -> EnvelopeKind {
        match self.id.nibble() {
            0 => EnvelopeKind::Ar,
            1 => EnvelopeKind::Asr,
            2 => EnvelopeKind::Atr,
            _ => EnvelopeKind::Lfo,
        }
    }
}

/// The rolloff record of one camera state of a 3D control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpatialRecord {
    pub info: u32,
    pub curves: u32,
    /// Near distance in bits 0 to 14, far distance in bits 16 to 30 (metres), for each azimuth quadrant.
    pub ranges: [u32; 4],
}

impl SpatialRecord {
    /// The camera state this record is for.
    pub fn camera(&self) -> u8 {
        ((self.info >> 24) & 0xF) as u8
    }
}

/// A 3D control: distance and azimuth to a rolloff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spatial {
    pub id: SourceId,
    pub records: Vec<SpatialRecord>,
}

/// A sub-mix channel: a clamped sum of levels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubChannel {
    pub upper: i32,
    pub lower: i32,
    pub inputs: Vec<SourceId>,
}

/// What a master channel's outputs mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    Volume,
    Pitch,
    Filter,
    Depth,
    Other,
}

/// One output of a master channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetWord {
    /// Output slot, 0 to 31.
    pub slot: u8,
    /// Which of the channel's 3D inputs it uses.
    pub spatial: u8,
    /// The output is the 3D control's azimuth.
    pub azimuth: bool,
    pub offset: i32,
}

/// A master channel: the sum of its inputs on top of a base level, written to the slots of one sound object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterChannel {
    /// The object it drives (a kind-2 id: state and object number).
    pub object: SourceId,
    /// The starting level.
    pub base: i32,
    pub inputs: Vec<SourceId>,
    pub kind: OutputKind,
    pub words: Vec<PresetWord>,
}

/// One state of the map: the elements of one kind of sound object.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    pub controls: Vec<Control>,
    pub events: Vec<Event>,
    pub spatial: Vec<Spatial>,
    pub subs: Vec<SubChannel>,
    pub masters: Vec<MasterChannel>,
}

/// A mixer map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixMap {
    /// The map type word of the header (0 in the shipped maps).
    pub map_type: i32,
    /// `None` for a state the file does not have.
    pub states: Vec<Option<State>>,
}

struct Reader<'a> {
    data: &'a [u8],
}

impl Reader<'_> {
    fn u32(&self, at: usize) -> Result<u32> {
        let end = at.checked_add(4).ok_or(Error::BadOffset(at as i64))?;
        let bytes = self.data.get(at..end).ok_or(Error::Truncated { at, needed: 4, have: self.data.len() })?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i32(&self, at: usize) -> Result<i32> {
        Ok(self.u32(at)? as i32)
    }

    /// An offset field: `base + value`, or `None` for -1.
    fn section(&self, base: usize, at: usize) -> Result<Option<usize>> {
        let value = self.i32(at)?;
        if value == -1 {
            return Ok(None);
        }
        let target = i64::from(value) + base as i64;
        if value < 0 || target as usize >= self.data.len() {
            return Err(Error::BadOffset(i64::from(value)));
        }
        Ok(Some(target as usize))
    }

    fn ids(&self, at: usize, count: usize) -> Result<Vec<SourceId>> {
        (0..count).map(|i| Ok(SourceId(self.u32(at + 4 * i)?))).collect()
    }

    /// A count whose elements need at least `min_size` bytes each from `at`.
    fn count(&self, at: usize, min_size: usize, mask: u32) -> Result<usize> {
        let n = self.u32(at)? & mask;
        let room = self.data.len().saturating_sub(at) / min_size.max(1);
        if n as usize > room {
            return Err(Error::BadCount(n));
        }
        Ok(n as usize)
    }
}

fn signed16(word: u32) -> i32 {
    i32::from(word as u16 as i16)
}

fn parse_controls(r: &Reader, at: usize) -> Result<Vec<Control>> {
    let n = r.count(at, 8, u32::MAX)?;
    let mut p = at + 16;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let (input, swing) = (SourceId(r.u32(p)?), r.u32(p + 4)?);
        let scales = ((swing >> 16) & 0x1F) as usize;
        out.push(Control { input, swing, scales: r.ids(p + 8, scales)? });
        p += 8 + 4 * scales;
    }
    Ok(out)
}

fn parse_events(r: &Reader, at: usize) -> Result<Vec<Event>> {
    let n = r.count(at, 24, u32::MAX)?;
    let mut p = at + 16;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let swing_word = r.u32(p + 4)?;
        let scales = ((swing_word >> 16) & 0xF) as usize;
        out.push(Event {
            id: SourceId(r.u32(p)?),
            swing: signed16(swing_word),
            trigger: SourceId(r.u32(p + 8)?),
            params: [r.u32(p + 12)?, r.u32(p + 16)?, r.u32(p + 20)?],
            scales: r.ids(p + 24, scales)?,
        });
        p += 24 + 4 * scales;
    }
    Ok(out)
}

fn parse_spatial(r: &Reader, at: usize) -> Result<Vec<Spatial>> {
    let n = r.count(at, 28, 0xFF)?;
    let mut p = at + 16;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let id = SourceId(r.u32(p)?);
        let states = usize::from(id.nibble());
        let mut records = Vec::with_capacity(states);
        for s in 0..states {
            let q = p + 4 + 24 * s;
            let ranges = [r.u32(q + 8)?, r.u32(q + 12)?, r.u32(q + 16)?, r.u32(q + 20)?];
            records.push(SpatialRecord { info: r.u32(q)?, curves: r.u32(q + 4)?, ranges });
        }
        out.push(Spatial { id, records });
        p += 4 + 24 * states;
    }
    Ok(out)
}

fn parse_subs(r: &Reader, at: usize) -> Result<Vec<SubChannel>> {
    let n = r.count(at, 8, u32::MAX)?;
    let mut p = at + 16;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let (header, limits) = (r.u32(p)?, r.u32(p + 4)?);
        let inputs = ((header >> 16) & 0xFF) as usize;
        out.push(SubChannel {
            upper: ((limits >> 16) & 0x7FFF) as i32,
            lower: signed16(limits),
            inputs: r.ids(p + 8, inputs)?,
        });
        p += 8 + 4 * inputs;
    }
    Ok(out)
}

fn output_kind(nibble: u32) -> OutputKind {
    match nibble {
        0 => OutputKind::Volume,
        1 => OutputKind::Pitch,
        2 => OutputKind::Filter,
        4 => OutputKind::Depth,
        _ => OutputKind::Other,
    }
}

fn parse_masters(r: &Reader, at: usize, presets: Option<usize>) -> Result<Vec<MasterChannel>> {
    let n = r.count(at, 12, u32::MAX)?;
    let mut preset = presets.ok_or(Error::BadOffset(-1))?;
    let mut p = at + 16;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let (header, data, object) = (r.u32(p)?, r.u32(p + 4)?, SourceId(r.u32(p + 8)?));
        let inputs = ((header >> 16) & 0xFF) as usize;
        let head = r.u32(preset)?;
        let count = (head & 0x1F) as usize;
        let mut words = Vec::with_capacity(count);
        for w in 0..count {
            let word = r.u32(preset + 4 + 4 * w)?;
            words.push(PresetWord {
                slot: ((word >> 26) & 0x1F) as u8,
                spatial: ((word >> 21) & 0x1F) as u8,
                azimuth: word >> 31 != 0,
                offset: signed16(word),
            });
        }
        out.push(MasterChannel {
            object,
            base: signed16(data >> 16),
            inputs: r.ids(p + 12, inputs)?,
            kind: output_kind((head >> 24) & 0xF),
            words,
        });
        preset += 4 + 4 * count;
        p += 12 + 4 * inputs;
    }
    Ok(out)
}

fn parse_state(r: &Reader, base: usize) -> Result<State> {
    let section = |i: usize| r.section(base, base + 4 * i);
    let mut state = State::default();
    if let Some(at) = section(1)? {
        state.controls = parse_controls(r, at)?;
    }
    if let Some(at) = section(2)? {
        state.spatial = parse_spatial(r, at)?;
    }
    if let Some(at) = section(3)? {
        state.subs = parse_subs(r, at)?;
    }
    if let Some(at) = section(4)? {
        state.masters = parse_masters(r, at, section(5)?)?;
    }
    if let Some(at) = section(6)? {
        state.events = parse_events(r, at)?;
    }
    Ok(state)
}

impl MixMap {
    /// Parses the bytes of a `.mxb` file.
    pub fn parse(data: &[u8]) -> Result<MixMap> {
        let r = Reader { data };
        let map_type = r.i32(0)?;
        let count = r.count(4, 4, u32::MAX)?;
        let table = r.i32(8)?;
        if table < 0 {
            return Err(Error::BadOffset(i64::from(table)));
        }
        let mut states = Vec::with_capacity(count);
        for s in 0..count {
            let Some(base) = r.section(0, table as usize + 4 * s)? else {
                states.push(None);
                continue;
            };
            states.push(Some(parse_state(&r, base)?));
        }
        Ok(MixMap { map_type, states })
    }

    /// The state `n`, if the file has it.
    pub fn state(&self, n: usize) -> Option<&State> {
        self.states.get(n)?.as_ref()
    }
}

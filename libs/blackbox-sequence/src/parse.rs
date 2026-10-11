//! Decode and validate tag references before exposing a package.

use std::collections::BTreeMap;

use blackbox_carp::{Carp, Record, tag4};

use crate::{Action, Engine, Error, Event, Package, Response, Result, State, System};

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

#[derive(Clone, Copy)]
struct Bytes<'a>(&'a [u8]);

impl<'a> Bytes<'a> {
    fn slice(self, at: usize, len: usize) -> Result<&'a [u8]> {
        at.checked_add(len)
            .and_then(|end| self.0.get(at..end))
            .ok_or_else(|| invalid(format!("range {at}+{len} exceeds {} bytes", self.0.len())))
    }

    fn word(self, at: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.slice(at, 4)?.try_into().unwrap()))
    }

    fn number(self, at: usize) -> Result<f32> {
        let value = f32::from_bits(self.word(at)?);
        if !value.is_finite() {
            return Err(invalid(format!("non-finite number at {at}")));
        }
        Ok(value)
    }

    fn count(self, at: usize, start: usize, stride: usize) -> Result<usize> {
        let count = self.word(at)? as usize;
        let len = count.checked_mul(stride).ok_or_else(|| invalid("array size overflows"))?;
        self.slice(start, len)?;
        Ok(count)
    }
}

struct Records<'a>(BTreeMap<u32, Record<'a>>);

impl<'a> Records<'a> {
    fn get(&self, tag: u32, kind: &[u8; 2]) -> Result<Bytes<'a>> {
        let prefix = u16::from_be_bytes(*kind) as u32;
        if tag >> 16 != prefix {
            return Err(invalid(format!("tag {tag:08X} has the wrong record type")));
        }
        self.0.get(&tag).map(|r| Bytes(r.data)).ok_or_else(|| invalid(format!("missing tag {tag:08X}")))
    }

    fn string(&self, tag: u32) -> Result<String> {
        let record = self.0.get(&tag).ok_or_else(|| invalid(format!("missing string {tag:08X}")))?;
        let end = record.data.iter().position(|&b| b == 0).ok_or_else(|| invalid("unterminated string"))?;
        std::str::from_utf8(&record.data[..end]).map(str::to_owned).map_err(|_| invalid("invalid UTF-8 string"))
    }

    fn engine(&self, record: &Record<'_>) -> Result<Engine> {
        let b = Bytes(record.data);
        let count = b.count(4, 8, 8)?;
        let mut systems = BTreeMap::new();
        for i in 0..count {
            let key = b.word(8 + i * 4)?;
            let system = self.system(b.word(8 + (count + i) * 4)?)?;
            if systems.insert(key, system).is_some() {
                return Err(invalid("duplicate system name"));
            }
        }
        Ok(Engine { name: self.string(b.word(0)?)?, name_hash: record.count, systems })
    }

    fn system(&self, tag: u32) -> Result<System> {
        let b = self.get(tag, b"eS")?;
        let count = b.count(8, 12, 8)?;
        let mut states = BTreeMap::new();
        for i in 0..count {
            let key = b.word(12 + i * 4)?;
            let state = self.state(b.word(12 + (count + i) * 4)?)?;
            if states.insert(key, state).is_some() {
                return Err(invalid("duplicate state name"));
            }
        }
        let (initial, default) = (b.word(0)?, b.word(4)?);
        if !states.contains_key(&default) {
            return Err(invalid("system has no default state"));
        }
        Ok(System { initial, default, states })
    }

    fn state(&self, tag: u32) -> Result<State> {
        let b = self.get(tag, b"eT")?;
        let count = b.count(0, 8, 12)?;
        let filter = b.word(4)?;
        if filter != 0 && !self.0.contains_key(&filter) {
            return Err(invalid("state has missing filter"));
        }
        let mut responses = BTreeMap::new();
        for i in 0..count {
            let at = 8 + count * 4 + i * 8;
            let action = b.word(at + 4)?;
            self.get(action, b"eA")?;
            let response = Response { end_state: b.word(at)?, action };
            if responses.insert(b.word(8 + i * 4)?, response).is_some() {
                return Err(invalid("duplicate stimulus"));
            }
        }
        Ok(State { filter: (filter != 0).then_some(filter), responses })
    }

    fn action(&self, record: &Record<'_>) -> Result<Action> {
        let b = Bytes(record.data);
        let tags_count = b.count(4, 16, 8)?;
        let times_count = b.count(8, 16 + tags_count * 8, 8)?;
        let lists_start = 16 + (tags_count + times_count) * 4;
        let list = |i| -> Result<u32> {
            let tag = b.word(lists_start + i * 4)?;
            self.get(tag, b"el")?;
            Ok(tag)
        };
        let mut tags = BTreeMap::new();
        for i in 0..tags_count {
            if tags.insert(b.word(16 + i * 4)?, list(i)?).is_some() {
                return Err(invalid("duplicate event tag"));
            }
        }
        let mut times = Vec::with_capacity(times_count);
        let mut last = f32::NEG_INFINITY;
        for i in 0..times_count {
            let time = b.number(16 + (tags_count + i) * 4)?;
            if time < last {
                return Err(invalid("action times are not ordered"));
            }
            times.push((time, list(tags_count + i)?));
            last = time;
        }
        Ok(Action { name_hash: b.word(0)?, duration: b.number(12)?, tags, times })
    }
}

impl Package {
    /// A CARP blob, starting at the root group, without a chunk/runtime header.
    pub fn parse(blob: &[u8]) -> Result<Self> {
        let carp = Carp::new(blob);
        let root = carp.root()?;
        if root.tag != tag4(b"CARP") || root.group_count() != 0 {
            return Err(invalid("expected a flat CARP event group"));
        }
        let mut records = Records(BTreeMap::new());
        for record in root.records() {
            let record = record?;
            if records.0.insert(record.tag, record).is_some() {
                return Err(invalid("duplicate record tag"));
            }
        }
        let mut package = Package::default();
        for record in records.0.values() {
            match (record.tag >> 16) as u16 {
                0x6545 => package.engines.push(records.engine(record)?),
                0x6541 => {
                    package.actions.insert(record.tag, records.action(record)?);
                }
                0x656C => {
                    package.event_lists.insert(record.tag, events(record.data)?);
                }
                0x694E | 0x6353 => {
                    package.strings.insert(record.tag, records.string(record.tag)?);
                }
                _ => {}
            }
        }
        Ok(package)
    }
}

fn events(bytes: &[u8]) -> Result<Vec<Event>> {
    let b = Bytes(bytes);
    let count = b.count(0, 16, 16)?;
    let descriptors_end = 16 + count * 16;
    (0..count)
        .map(|i| {
            let at = 16 + i * 16;
            let id = b.word(at)?;
            let size = b.word(at + 4)? as usize;
            let start = at.checked_add(b.word(at + 8)? as usize).ok_or_else(|| invalid("event offset overflows"))?;
            if size < 4 || start < descriptors_end || b.word(start)? != id {
                return Err(invalid("invalid event static-data range or repeated ID"));
            }
            Ok(Event { id, data: b.slice(start, size)?.to_vec() })
        })
        .collect()
}

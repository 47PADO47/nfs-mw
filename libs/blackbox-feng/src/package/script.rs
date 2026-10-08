use super::raw::{tags, words};
use super::types::*;
use crate::error::{Error, Result};

/// Reads one `Scrp` chunk.
pub fn read_script(data: &[u8]) -> Result<Script> {
    let mut script = Script::default();
    let mut current: Option<TrackBuilder> = None;
    let mut done: Vec<Track> = Vec::new();
    for t in tags(data, "Scrp")? {
        match &t.id {
            b"Sh" => {
                if t.data.len() < 16 {
                    return Err(Error::Malformed("Sh", format!("{} bytes", t.data.len())));
                }
                let word = |i: usize| u32::from_le_bytes(t.data[i * 4..i * 4 + 4].try_into().unwrap_or([0; 4]));
                script.id = word(0);
                script.length = word(1) as i32;
                script.flags = word(2);
            }
            b"Sc" => script.chain = t.u32().filter(|c| *c != 0),
            b"FI" => {
                if let Some(b) = current.take() {
                    done.push(b.finish());
                }
                if t.data.len() < 8 {
                    return Err(Error::Malformed("FI", format!("{} bytes", t.data.len())));
                }
                let param = ParamType::from_id(t.data[0]);
                let interp = match t.data[2] {
                    0 => Interp::None,
                    1 => Interp::Linear,
                    3 => Interp::MoveTo,
                    _ => Interp::Unsupported,
                };
                let length = u32::from_le_bytes([t.data[4], t.data[5], t.data[6], t.data[7]]) as i32;
                current = param.map(|param| TrackBuilder {
                    param,
                    interp,
                    action: t.data[3],
                    length,
                    offset: 0,
                    base: None,
                    keys: Vec::new(),
                });
            }
            b"To" => {
                if let (Some(b), Some(v)) = (current.as_mut(), t.u32()) {
                    b.offset = v as usize;
                }
            }
            b"Kd" => {
                if let Some(b) = current.as_mut() {
                    b.add_keys(t.data);
                }
            }
            b"EV" => {
                let mut w = words(t.data);
                while let (Some(message), Some(target), Some(time)) = (w.next(), w.next(), w.next()) {
                    script.events.push(Event { message, target, time });
                }
            }
            _ => {}
        }
    }
    if let Some(b) = current.take() {
        done.push(b.finish());
    }
    script.tracks = done;
    Ok(script)
}

struct TrackBuilder {
    param: ParamType,
    interp: Interp,
    action: u8,
    length: i32,
    offset: usize,
    base: Option<Key>,
    keys: Vec<Key>,
}

impl TrackBuilder {
    /// Keys are `{i32 time, value}` back to back; the first one (time -1) is the base.
    fn add_keys(&mut self, data: &[u8]) {
        let comps = self.param.components();
        let size = 4 + comps * 4;
        for raw in data.chunks_exact(size) {
            let time = i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
            let mut value = [0u32; 4];
            for (i, v) in value.iter_mut().enumerate().take(comps) {
                *v = u32::from_le_bytes([raw[4 + i * 4], raw[5 + i * 4], raw[6 + i * 4], raw[7 + i * 4]]);
            }
            let key = Key { time, value };
            if self.base.is_none() && time < 0 {
                self.base = Some(key);
            } else {
                self.keys.push(key);
            }
        }
    }

    fn finish(self) -> Track {
        // Without an explicit base key the first key is the base and nothing is added to it.
        let base = self.base.unwrap_or(Key { time: -1, value: [0; 4] });
        Track {
            param: self.param,
            interp: self.interp,
            action: self.action,
            length: self.length,
            offset: self.offset,
            base,
            keys: self.keys,
        }
    }
}

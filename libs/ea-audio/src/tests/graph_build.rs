//! Builds synthetic `.mpf` files with a node graph.

/// A node to write: first word, router, transitions `(lo, hi, target)` and the fourth word.
pub struct N {
    pub id: u16,
    pub section: u8,
    pub router: u16,
    pub trans: Vec<(i8, i8, i16)>,
    pub d3: u32,
}

/// An audio node playing stream `stream` and then `next` (for every control value).
pub fn audio(stream: u16, next: i16) -> N {
    N { id: stream + 1, section: 5, router: 0, trans: vec![(0, 127, next)], d3: 0 }
}

/// A group head (id 0), end node (0xFFFF) or fire-event node (0xFFFD) with its transitions.
pub fn control(id: u16, trans: Vec<(i8, i8, i16)>, d3: u32) -> N {
    N { id, section: 5, router: 0, trans, d3 }
}

/// An event: 24-bit id and its actions `(w1, w2)`.
pub struct E {
    pub id: u32,
    pub actions: Vec<(u32, u32)>,
}

/// The two-action event of a song: stop everything, then branch to `node`.
pub fn song_event(id: u32, node: u32) -> E {
    E { id, actions: vec![(0x000F_0400, 0x01FF_FFFF), (0x0003_0400, 0x00FF_0000 | node)] }
}

#[derive(Default)]
pub struct Map {
    pub nodes: Vec<N>,
    pub events: Vec<E>,
    /// Routers as lists of `(key, value)`.
    pub routers: Vec<Vec<(u16, u16)>>,
    pub variables: Vec<(&'static str, u32)>,
    /// `(raw offset, duration ms)` of each stream.
    pub streams: Vec<(u32, u32)>,
}

fn put16(v: &mut [u8], at: usize, x: u16) {
    v[at..at + 2].copy_from_slice(&x.to_le_bytes());
}

fn put32(v: &mut [u8], at: usize, x: u32) {
    v[at..at + 4].copy_from_slice(&x.to_le_bytes());
}

impl Map {
    pub fn build(&self) -> Vec<u8> {
        let mut v = vec![0u8; 0x40];
        v[..4].copy_from_slice(b"xDFP");
        v[4] = 5;
        v[5] = 1;
        v[0x0D] = 1;
        v[0x0E] = 7;
        v[0x0F] = self.events.len() as u8;
        v[0x10] = self.routers.len() as u8;
        v[0x11] = self.variables.len() as u8;
        put16(&mut v, 0x12, self.nodes.len() as u16);

        // Nodes: the offset table first, then the records.
        let table = v.len();
        put32(&mut v, 0x14, table as u32);
        v.resize(table + 2 * self.nodes.len(), 0);
        align(&mut v);
        for (i, n) in self.nodes.iter().enumerate() {
            let at = v.len();
            put16(&mut v, table + 2 * i, (at / 4) as u16);
            let d0 = u32::from(n.id) | (u32::from(n.section) << 21);
            let d1 = u32::from(n.router) | ((n.trans.len() as u32) << 12) | (4 << 20) | (1 << 24);
            for word in [d0, d1, 0, n.d3] {
                v.extend_from_slice(&word.to_le_bytes());
            }
            for &(lo, hi, target) in &n.trans {
                v.extend_from_slice(&[lo as u8, hi as u8]);
                v.extend_from_slice(&target.to_le_bytes());
            }
        }

        // Events: offset table, then 20 + 12n bytes each.
        let table = v.len();
        put32(&mut v, 0x1C, table as u32);
        v.resize(table + 2 * self.events.len(), 0);
        align(&mut v);
        for (i, e) in self.events.iter().enumerate() {
            let at = v.len();
            put16(&mut v, table + 2 * i, (at / 4) as u16);
            v.extend_from_slice(&[0; 12]);
            v.extend_from_slice(&(((e.actions.len() as u32) << 24) | e.id).to_le_bytes());
            v.extend_from_slice(&[0; 4]);
            for &(w1, w2) in &e.actions {
                for word in [0x1100_0001, w1, w2] {
                    v.extend_from_slice(&word.to_le_bytes());
                }
            }
        }

        // Variables: 16-byte name, a word of junk, then the initial value is at +0x10.
        let at = v.len() as u32;
        put32(&mut v, 0x24, at);
        for &(name, initial) in &self.variables {
            let mut record = [0u8; 20];
            record[..name.len()].copy_from_slice(name.as_bytes());
            record[16..].copy_from_slice(&initial.to_le_bytes());
            v.extend_from_slice(&record);
        }

        // Routers: count + 1 offsets (in u32 units), then the entries.
        let table = v.len();
        put32(&mut v, 0x28, table as u32);
        v.resize(table + 4 * (self.routers.len() + 1), 0);
        let mut offsets = vec![v.len() / 4];
        for entries in &self.routers {
            for &(key, value) in entries {
                v.extend_from_slice(&((u32::from(key) << 16) | u32::from(value)).to_le_bytes());
            }
            offsets.push(v.len() / 4);
        }
        for (i, o) in offsets.iter().enumerate() {
            put32(&mut v, table + 4 * i, *o as u32);
        }

        // One track, and the stream table.
        let tracks_table = v.len();
        put32(&mut v, 0x2C, tracks_table as u32);
        v.extend_from_slice(&[0; 4]);
        let entry = v.len();
        put32(&mut v, tracks_table, (entry / 4) as u32);
        v.extend_from_slice(&[0; 16]);
        let samples = v.len();
        put32(&mut v, 0x34, samples as u32);
        for &(raw, ms) in &self.streams {
            v.extend_from_slice(&raw.to_le_bytes());
            v.extend_from_slice(&ms.to_le_bytes());
        }
        let end = v.len() as u32;
        put32(&mut v, 0x38, end);
        v
    }
}

fn align(v: &mut Vec<u8>) {
    while !v.len().is_multiple_of(4) {
        v.push(0);
    }
}

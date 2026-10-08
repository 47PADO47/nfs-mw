//! Builders for synthetic CARP blobs, collision packs, grids and bounds, laid out like the real
//! files (`docs/formats/collision.md`).

use crate::carp::{tag, tag4};

pub fn push_f32(v: &mut Vec<u8>, x: f32) {
    v.extend_from_slice(&x.to_le_bytes());
}

pub fn push_u16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}

pub fn push_u32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}

pub fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

/// A record to put in a group: tag, element count, bytes.
pub type Rec = (u32, u32, Vec<u8>);

/// A CARP blob: root `CARP` group with one child group `child` holding `records`.
pub fn carp(child: u32, records: &[Rec]) -> Vec<u8> {
    let mut b = Vec::new();
    // Root group: embedded, one child group, array right after the header.
    for w in [tag4(b"CARP"), 0b10 | (1 << 5), 0, 1] {
        push_u32(&mut b, w);
    }
    // Child group.
    for w in [child, 0b10, records.len() as u32, 1] {
        push_u32(&mut b, w);
    }
    let headers_end = b.len() + records.len() * 16;
    let mut data = Vec::new();
    for (i, (t, count, bytes)) in records.iter().enumerate() {
        let header_at = 0x20 + i * 16;
        let at = (headers_end + data.len()).next_multiple_of(16);
        data.resize(at - headers_end, 0xAA);
        data.extend_from_slice(bytes);
        for w in [*t, 0b10 | (bytes.len() as u32) << 8, *count, (at - header_at) as u32] {
            push_u32(&mut b, w);
        }
    }
    b.extend_from_slice(&data);
    b
}

/// The payload of a `0x3B801` chunk (what `aligned_payload(16)` returns): header plus blob.
pub fn pack_payload(section: u32, blob: &[u8]) -> Vec<u8> {
    let mut blob = blob.to_vec();
    blob.resize(blob.len().next_multiple_of(16), 0xAA);
    let mut v = Vec::new();
    for w in [blob.len() as u32, section, 0, 0] {
        push_u32(&mut v, w);
    }
    v.extend_from_slice(&blob);
    v
}

/// A whole `0x3B801` chunk positioned so the payload needs 8 bytes of alignment padding.
pub fn pack_chunk(section: u32, blob: &[u8]) -> Vec<u8> {
    let mut payload = vec![0x11; 8];
    payload.extend_from_slice(&pack_payload(section, blob));
    chunk(0x3B801, &payload)
}

/// A packed strip vertex.
pub fn vert(x: i16, y: i16, z: i16, surface: u8, flags: u8) -> [u8; 8] {
    let mut v = Vec::new();
    for c in [x, y, z] {
        v.extend_from_slice(&c.to_le_bytes());
    }
    v.extend_from_slice(&[surface, flags]);
    v.try_into().unwrap()
}

/// A strip: centre, radius (1/16 m), strip flags, and the real vertices (the first two get the
/// vertex count and flags written into their surface bytes).
pub struct StripSpec {
    pub center: [f32; 3],
    pub radius: u16,
    pub flags: u16,
    pub verts: Vec<[u8; 8]>,
}

pub struct BarrierSpec {
    pub p0: [f32; 3],
    pub p1: [f32; 3],
    pub surface: u8,
    pub flags: u8,
}

pub fn article(strips: &[StripSpec], barriers: &[BarrierSpec], surfaces: &[u32]) -> Vec<u8> {
    let spheres = strips.len() * 16;
    let mut strip_data = Vec::new();
    let mut offsets = Vec::new();
    for s in strips {
        offsets.push((spheres + strip_data.len()) as u16);
        for (i, v) in s.verts.iter().enumerate() {
            let mut v = *v;
            match i {
                0 => v[6..8].copy_from_slice(&(s.verts.len() as u16).to_le_bytes()),
                1 => v[6..8].copy_from_slice(&s.flags.to_le_bytes()),
                _ => {}
            }
            strip_data.extend_from_slice(&v);
        }
    }
    let strips_size = (spheres + strip_data.len()).next_multiple_of(16);
    let mut a = Vec::new();
    push_u16(&mut a, strips.len() as u16);
    push_u16(&mut a, strips_size as u16);
    push_u16(&mut a, barriers.len() as u16);
    push_u16(&mut a, (barriers.len() * 32) as u16);
    a.extend_from_slice(&[0, surfaces.len() as u8]);
    push_u16(&mut a, (surfaces.len() * 4) as u16);
    push_u16(&mut a, 7);
    push_u16(&mut a, 0);
    for (s, off) in strips.iter().zip(&offsets) {
        for c in s.center {
            push_f32(&mut a, c);
        }
        push_u16(&mut a, s.radius);
        push_u16(&mut a, *off);
    }
    a.extend_from_slice(&strip_data);
    a.resize(16 + strips_size, 0);
    for b in barriers {
        for c in b.p0 {
            push_f32(&mut a, c);
        }
        // The surface byte pair sits in the w of point 0.
        a.extend_from_slice(&[b.surface, b.flags, 0, 0]);
        for c in b.p1 {
            push_f32(&mut a, c);
        }
        let len = ((b.p1[0] - b.p0[0]).powi(2) + (b.p1[2] - b.p0[2]).powi(2)).sqrt();
        push_f32(&mut a, 1.0 / len);
    }
    for s in surfaces {
        push_u32(&mut a, *s);
    }
    a.resize(a.len().next_multiple_of(16), 0xAA);
    a
}

/// An instance record: axis-aligned unless `rows` says otherwise.
pub struct InstanceSpec {
    pub row_x: [f32; 3],
    pub row_z: [f32; 3],
    pub half_width: f32,
    pub half_length: f32,
    pub half_height: f32,
    /// World position of the centre (the file stores its negation, rotated).
    pub position: [f32; 3],
    pub radius: f32,
    pub flags: u16,
    pub group: u16,
    pub article: u16,
}

impl InstanceSpec {
    pub fn upright(position: [f32; 3], half: [f32; 3], article: u16) -> Self {
        Self {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            half_width: half[0],
            half_length: half[2],
            half_height: half[1],
            position,
            radius: (half[0] * half[0] + half[2] * half[2]).sqrt(),
            flags: 0,
            group: 0,
            article,
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        for c in self.row_x {
            push_f32(&mut v, c);
        }
        push_f32(&mut v, self.half_width);
        push_u16(&mut v, 1234);
        push_u16(&mut v, self.flags);
        push_f32(&mut v, self.half_height);
        push_u16(&mut v, self.group);
        push_u16(&mut v, self.article);
        push_u32(&mut v, 0xDEAD_0000);
        for c in self.row_z {
            push_f32(&mut v, c);
        }
        push_f32(&mut v, self.half_length);
        // Third row is the derived y; only upright instances are built here.
        let (x, z, p) = (self.row_x, self.row_z, self.position);
        let y = if self.flags & 3 != 0 { crate::math::cross(z, x) } else { [0.0, 1.0, 0.0] };
        for i in 0..3 {
            push_f32(&mut v, -(p[0] * x[i] + p[1] * y[i] + p[2] * z[i]));
        }
        push_f32(&mut v, self.radius);
        v
    }
}

/// A collision pack payload (header plus blob) with the given articles and instances.
pub fn pack(section: u32, articles: &[Vec<u8>], instances: &[InstanceSpec]) -> Vec<u8> {
    let mut records: Vec<Rec> = vec![(tag4(b"Name"), 0, vec![0; 4])];
    for (i, a) in articles.iter().enumerate() {
        records.push((tag(*b"ca", i as u16), 1, a.clone()));
    }
    let inst_bytes: Vec<u8> = instances.iter().flat_map(InstanceSpec::bytes).collect();
    records.push((tag(*b"ci", 0), instances.len() as u32, inst_bytes));
    pack_payload(section, &carp(tag4(b"Arti"), &records))
}

/// A flat 4-vertex strip on the plane y = 0 covering x, z in [-4, 4] around its centre.
pub fn ground_strip(surface: u8) -> StripSpec {
    StripSpec {
        center: [0.0; 3],
        radius: 8 * 16,
        flags: 0,
        verts: vec![
            vert(-4 * 128, 0, -4 * 128, 0, 0),
            vert(-4 * 128, 0, 4 * 128, 0, 0),
            vert(4 * 128, 0, -4 * 128, surface, 0),
            vert(4 * 128, 0, 4 * 128, surface, 0),
        ],
    }
}

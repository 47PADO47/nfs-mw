//! Byte builders for synthetic scene chunks, laid out as docs/specs/nis-scenes.md sections 2 to 4 describe them.

/// A chunk: `u32 id`, `u32 size`, then the payload.
pub fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + payload.len());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// A container chunk: the children's bytes are its payload. `id` must carry the container bit.
pub fn container(id: u32, children: &[Vec<u8>]) -> Vec<u8> {
    chunk(id, &children.concat())
}

/// The scene header: the 64-byte fixed part, then the description and its NUL, padded to 4 bytes.
/// `fields` are the six `i32` fields at 0x18 to 0x2C: scene type, layout, car animation, cars, start, vanish.
pub fn header(name: &str, name_hash: u32, fields: [i32; 6], overlay: &str, description: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&name_hash.to_le_bytes());
    out.extend_from_slice(&fixed(name, 16));
    out.extend_from_slice(&0u32.to_le_bytes());
    for field in fields {
        out.extend_from_slice(&field.to_le_bytes());
    }
    out.extend_from_slice(&fixed(overlay, 16));
    out.extend_from_slice(description.as_bytes());
    out.push(0);
    while out.len() % 4 != 0 {
        out.push(0);
    }
    out
}

/// A character record: 8 bytes of `0x11` padding, then the 144-byte data. The local matrix is the identity with the
/// translation `at` in floats 12 to 14. LOD names are `lod`, and the clip and skeleton are the given hashes.
pub fn character(type_id: u32, instance: u32, at: [f32; 3], lod: [u32; 4], clip: u32, skeleton: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity(144);
    push_u32(&mut data, type_id);
    push_u32(&mut data, instance);
    push_u32(&mut data, 0); // parent: the scene root
    push_u32(&mut data, 0x20); // play flags
    for value in [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, at[0], at[1], at[2], 1.0] {
        push_f32(&mut data, value);
    }
    for hash in lod {
        push_u32(&mut data, hash);
    }
    for _ in 0..3 {
        push_u32(&mut data, 0); // translation, quaternion and scale animations
    }
    push_u32(&mut data, clip);
    push_u32(&mut data, skeleton);
    push_f32(&mut data, 1.0); // play speed
    push_f32(&mut data, 0.0); // play delay
    push_u32(&mut data, 0); // world object type
    push_u32(&mut data, 0); // loop start
    push_u32(&mut data, 0); // loop end
    data.extend_from_slice(&[0u8; 8]); // unused
    assert_eq!(data.len(), 144, "the record data is 144 bytes");
    let mut out = vec![0x11u8; 8];
    out.extend_from_slice(&data);
    out
}

fn fixed(text: &str, len: usize) -> Vec<u8> {
    let mut out = text.as_bytes().to_vec();
    out.resize(len, 0);
    out
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_le_bytes());
}

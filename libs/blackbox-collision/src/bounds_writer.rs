//! The write side of `BoundsPack` (`0x8003B900`): a [`BoundsSet`] back into the bytes [`BoundsSet::parse`] reads.
//! Spec: `docs/formats/collision.md` ("Bounds").
//!
//! Positions and half extents are stored in whole millimetres, the orientation as a quaternion over 32767, so a set
//! read back from the bytes this writes has every length rounded to the millimetre. Writing that set again gives the
//! same bytes.

use blackbox_chunk::{ChunkWriter, ids};

use crate::bounds::{HEADER_LEN, NODE_LEN, QUAT_SCALE, VECTOR_SCALE};
use crate::bytes::malformed;
use crate::{Bounds, BoundsSet, Result};

/// The same limits the parser applies: a set it would refuse is not written.
const MAX_NODES: usize = 4096;
const MAX_CLOUDS: usize = 256;
/// Chunk payload alignment of a bounds set.
const PACK_ALIGN: usize = 16;

impl BoundsSet {
    /// The payload of a `0x3B901` chunk: header, nodes, point clouds. Fails when a length does not fit its 16-bit
    /// millimetre field (32.767 m), a value is not finite, or the tree is not one [`BoundsSet::parse`] accepts.
    pub fn to_payload(&self) -> Result<Vec<u8>> {
        let count = self.nodes.len();
        if count == 0 || count > MAX_NODES {
            return Err(malformed("bounds writer", format!("{count} nodes")));
        }
        if self.point_clouds.len() > MAX_CLOUDS {
            return Err(malformed("bounds writer", format!("{} point clouds", self.point_clouds.len())));
        }
        let mut out = Vec::with_capacity(HEADER_LEN + count * NODE_LEN);
        out.extend_from_slice(&self.name_hash.to_le_bytes());
        out.extend_from_slice(&(count as i32).to_le_bytes());
        out.resize(HEADER_LEN, 0);
        for (i, node) in self.nodes.iter().enumerate() {
            encode_node(&mut out, i, node, count, self.point_clouds.len())?;
        }
        let mut tail = vec![0u8; HEADER_LEN];
        tail[..4].copy_from_slice(&(self.point_clouds.len() as u32).to_le_bytes());
        out.extend_from_slice(&tail);
        for (i, cloud) in self.point_clouds.iter().enumerate() {
            let mut descriptor = vec![0u8; HEADER_LEN];
            descriptor[..4].copy_from_slice(&(cloud.len() as u32).to_le_bytes());
            out.extend_from_slice(&descriptor);
            for point in cloud {
                if point.iter().any(|v| !v.is_finite()) {
                    return Err(malformed("bounds writer", format!("point cloud {i} has a point that is not finite")));
                }
                for v in point {
                    out.extend_from_slice(&v.to_le_bytes());
                }
                out.extend_from_slice(&0f32.to_le_bytes());
            }
        }
        Ok(out)
    }
}

/// A `BoundsPack` chunk holding `sets`, one `0x3B901` child each with its payload aligned to 16 bytes, the way the
/// pack in `GlobalB.lzc` is laid out. The result is the whole file when the pack is its only chunk.
pub fn write_bounds_pack(sets: &[BoundsSet]) -> Result<Vec<u8>> {
    let payloads = sets.iter().map(BoundsSet::to_payload).collect::<Result<Vec<_>>>()?;
    let mut writer = ChunkWriter::new();
    writer.container(ids::BOUNDS_PACK, |w| {
        for payload in &payloads {
            w.leaf_aligned(ids::BOUNDS, PACK_ALIGN, payload);
        }
    });
    writer.finish().map_err(|e| malformed("bounds writer", e.to_string()))
}

fn encode_node(out: &mut Vec<u8>, index: usize, node: &Bounds, nodes: usize, clouds: usize) -> Result<()> {
    let what = |field: &str| format!("node {index}: {field}");
    let mm = |field: &str, v: f32| -> Result<i16> {
        let scaled = (v * VECTOR_SCALE).round();
        if !scaled.is_finite() || scaled < f32::from(i16::MIN) || scaled > f32::from(i16::MAX) {
            return Err(malformed("bounds writer", what(&format!("{field} {v} m does not fit 16-bit millimetres"))));
        }
        Ok(scaled as i16)
    };
    let quat = |v: f32| -> Result<i16> {
        if !v.is_finite() || v.abs() > 1.0 + 1e-3 {
            return Err(malformed("bounds writer", what(&format!("orientation {v} is not a unit quaternion part"))));
        }
        Ok((v * QUAT_SCALE).round().clamp(-QUAT_SCALE, QUAT_SCALE) as i16)
    };
    let first = usize::try_from(node.first_child).unwrap_or(usize::MAX);
    if node.child_count > 0 && first.saturating_add(usize::from(node.child_count)) > nodes {
        return Err(malformed("bounds writer", what("children run past the node array")));
    }
    let cloud = match node.point_cloud {
        None => 0xFFu8,
        Some(c) if usize::from(c) < clouds && c < 0x80 => c,
        Some(c) => return Err(malformed("bounds writer", what(&format!("point cloud {c} of {clouds}")))),
    };
    if !node.radius.is_finite() {
        return Err(malformed("bounds writer", what("radius is not finite")));
    }
    for v in node.orientation {
        out.extend_from_slice(&quat(v)?.to_le_bytes());
    }
    for (name, v) in ["position"; 3].into_iter().zip(node.position) {
        out.extend_from_slice(&mm(name, v)?.to_le_bytes());
    }
    out.extend_from_slice(&node.flags.to_le_bytes());
    for (name, v) in ["half dimension"; 3].into_iter().zip(node.half_dimensions) {
        out.extend_from_slice(&mm(name, v)?.to_le_bytes());
    }
    out.push(node.child_count);
    out.push(cloud);
    for (name, v) in ["pivot"; 3].into_iter().zip(node.pivot) {
        out.extend_from_slice(&mm(name, v)?.to_le_bytes());
    }
    out.extend_from_slice(&node.first_child.to_le_bytes());
    out.extend_from_slice(&node.radius.to_le_bytes());
    out.extend_from_slice(&node.surface.to_le_bytes());
    out.extend_from_slice(&node.name_hash.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests;

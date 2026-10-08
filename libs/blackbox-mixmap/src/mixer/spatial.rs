//! 3D controls: distance and azimuth to a rolloff. Spec: `docs/specs/dynamic-mixer.md` §6.

use super::{Mixer, SpatialNode};
use crate::map::SpatialRecord;
use crate::shape::{SILENCE_DB, UNITY, curve, db_from_q15};

/// Picks the record of each control for `camera`, else its default (camera 0), else the first.
pub(super) fn select_camera(nodes: &mut [SpatialNode], camera: u8) {
    for node in nodes {
        let find = |c: u8| node.records.iter().position(|r| r.camera() == c);
        node.current = find(camera).or_else(|| find(0)).unwrap_or(0);
    }
}

pub(super) fn update(m: &mut Mixer) {
    for i in 0..m.spatial.len() {
        let node = &m.spatial[i];
        let block = node.block.map(|src| m.graph.read(src));
        let [to_car, to_camera, azimuth_car, azimuth_camera, flags] = block;
        m.graph.spatial_azimuth[i] = 0;
        let (db, q) = match node.records.get(node.current) {
            Some(record) if flags & 1 != 0 => {
                m.graph.spatial_azimuth[i] = azimuth_of(record, azimuth_car, azimuth_camera);
                rolloff(record, to_car, to_camera, azimuth_car, azimuth_camera)
            }
            _ => (SILENCE_DB, 0),
        };
        m.graph.spatial_db[i] = db;
        m.graph.spatial_q[i] = q;
    }
}

fn azimuth_of(record: &SpatialRecord, to_car: i32, to_camera: i32) -> i32 {
    match (record.info >> 8) & 0xF {
        0 => to_camera,
        1 => to_car,
        _ => 0,
    }
}

/// `(dB, Q15)` of the rolloff for the distances (cm) and azimuths published.
fn rolloff(record: &SpatialRecord, to_car: i32, to_camera: i32, azimuth_car: i32, azimuth_camera: i32) -> (i32, i32) {
    let distance = match (record.info >> 12) & 0xF {
        0 => to_camera as f32 * 0.01,
        1 => to_car as f32 * 0.01,
        _ => -1.0,
    };
    let azimuth = azimuth_of(record, azimuth_car, azimuth_camera);
    let quad = ((azimuth as u32 >> 14) & 3) as usize;
    let next = (quad + 1) & 3;
    // The shape nibble of each quadrant: bits 28, 16, 24, 20.
    let shape = ((record.curves >> [28, 16, 24, 20][quad]) & 0xF) as u8;
    let within = azimuth - (quad as i32) * 0x4000;
    let range = |q: usize| ((record.ranges[q] & 0x7FFF) as f32, ((record.ranges[q] >> 16) & 0x7FFF) as f32);
    let ((min0, max0), (min1, max1)) = (range(quad), range(next));
    if distance > max0 && distance > max1 {
        return (SILENCE_DB, 0);
    }
    let fraction = |min: f32, max: f32| {
        if max <= min {
            return 0.0;
        }
        (distance.clamp(min, max) - min) / (max - min)
    };
    let (x0, x1) = ((fraction(min0, max0) * 32767.0) as i32, (fraction(min1, max1) * 32767.0) as i32);
    let first = curve(shape, x0);
    let second = if within != 0 { curve(shape, x1) } else { UNITY };
    let mix = within << 1;
    let q = ((first * (UNITY - mix)) >> 15) + ((second * mix) >> 15);
    (db_from_q15(q), q)
}

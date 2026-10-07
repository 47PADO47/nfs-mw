//! Synthetic `VisibleSectionManager` chunks in the MW layout.

use blackbox_chunk::ids;

use super::*;
use crate::Error;
use crate::layout::MOST_WANTED_VISIBLE;

const NODE: [u8; 8] = [0x0B, 0, 0, 0, 0x0B, 0, 0, 0];

fn chunk(id: u32, payload: &[u8]) -> Vec<u8> {
    let mut v = id.to_le_bytes().to_vec();
    v.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    v.extend_from_slice(payload);
    v
}

fn boundary(section: i16, points: &[[f32; 2]], panorama: bool) -> Vec<u8> {
    let mut r = NODE.to_vec();
    r.extend_from_slice(&section.to_le_bytes());
    r.extend_from_slice(&[points.len() as u8, u8::from(panorama)]);
    let min = [0, 1].map(|i| points.iter().map(|p| p[i]).fold(f32::MAX, f32::min));
    let max = [0, 1].map(|i| points.iter().map(|p| p[i]).fold(f32::MIN, f32::max));
    let centre = [0, 1].map(|i| (min[i] + max[i]) / 2.0);
    for v in [min, max, centre].into_iter().chain(points.iter().copied()) {
        r.extend(v.iter().flat_map(|f| f.to_le_bytes()));
    }
    r
}

fn drivable(section: i16, visible: &[i16], max: u8) -> Vec<u8> {
    let mut r = NODE.to_vec();
    r.extend_from_slice(&[0; 4]); // pBoundary
    r.extend_from_slice(&section.to_le_bytes());
    r.extend_from_slice(&[visible.len() as u8, max]);
    r.extend_from_slice(&(visible.len() as i16).to_le_bytes());
    for i in 0..usize::from(max) {
        r.extend_from_slice(&visible.get(i).copied().unwrap_or(0).to_le_bytes());
    }
    r.extend_from_slice(&[0; 2]);
    r
}

fn loading(name: &str, drivable: &[i16], extra: &[i16]) -> Vec<u8> {
    let mut r = vec![0u8; 0x4C];
    r[..8].copy_from_slice(&NODE);
    r[8..8 + name.len()].copy_from_slice(name.as_bytes());
    r[0x18..0x1A].copy_from_slice(&(drivable.len() as i16).to_le_bytes());
    for (i, s) in drivable.iter().enumerate() {
        r[0x1A + 2 * i..0x1C + 2 * i].copy_from_slice(&s.to_le_bytes());
    }
    r[0x3A..0x3C].copy_from_slice(&(extra.len() as i16).to_le_bytes());
    for (i, s) in extra.iter().enumerate() {
        r[0x3C + 2 * i..0x3E + 2 * i].copy_from_slice(&s.to_le_bytes());
    }
    r
}

fn square(x: f32, y: f32, size: f32) -> Vec<[f32; 2]> {
    vec![[x, y], [x + size, y], [x + size, y + size], [x, y + size]]
}

/// Zones A1 (0..100 square) and A2 (100..200), a panorama A90, a loading section
/// grouping A2 with A3, and an unreachable zone A5 outside the region.
fn sample() -> Vec<u8> {
    let mut info = 40i32.to_le_bytes().to_vec();
    info.extend_from_slice(&3i32.to_le_bytes());
    for i in 0..400 {
        info.extend_from_slice(&[101i16, 102, 103].get(i).copied().unwrap_or(0).to_le_bytes());
    }
    let boundaries = [
        boundary(101, &square(0.0, 0.0, 100.0), false),
        boundary(102, &square(100.0, 0.0, 100.0), false),
        boundary(103, &square(200.0, 0.0, 100.0), false),
        boundary(105, &square(0.0, 300.0, 100.0), false),
        boundary(190, &square(-500.0, -500.0, 1000.0), true),
    ]
    .concat();
    let drivables = [
        drivable(101, &[101, 102, 141, 142, 190, 2201], 12),
        drivable(102, &[101, 102, 141, 142], 4),
        drivable(103, &[103, 143], 2),
        drivable(105, &[105, 145], 2),
    ]
    .concat();
    let loadings = loading("CT1", &[102, 103], &[190, 104]);
    let body = [
        chunk(ids::VISIBLE_SECTION_MANAGER_INFO, &info),
        chunk(ids::VISIBLE_SECTION_BOUNDARIES, &boundaries),
        chunk(ids::LOADING_SECTIONS, &loadings),
        chunk(ids::DRIVABLE_SCENERY_SECTIONS, &drivables),
    ]
    .concat();
    chunk(ids::VISIBLE_SECTION_MANAGER, &body)
}

#[test]
fn reads_tables() {
    let v = read_visible_sections(&sample(), &MOST_WANTED_VISIBLE).unwrap();
    assert_eq!(v.lod_offset, 40);
    assert_eq!(v.region, [101, 102, 103]);
    assert_eq!(v.boundaries.len(), 5);
    let b = v.boundary(102).unwrap();
    assert_eq!((b.bbox_min, b.bbox_max, b.centre), ([100.0, 0.0], [200.0, 100.0], [150.0, 50.0]));
    assert_eq!(b.points.len(), 4);
    assert!(v.boundary(190).unwrap().panorama && !b.panorama);
    assert_eq!(v.drivable.len(), 4);
    assert_eq!(v.drivable(101).unwrap().visible, [101, 102, 141, 142, 190, 2201]);
    assert_eq!(v.drivable(103).unwrap().visible, [103, 143]);
    assert_eq!(v.loading[0].name, "CT1");
    assert_eq!((v.loading[0].drivable.as_slice(), v.loading[0].extra.as_slice()), (&[102, 103][..], &[190, 104][..]));
    assert!(v.in_region(102) && !v.in_region(105));
}

#[test]
fn missing_or_truncated_tables_are_errors() {
    assert!(matches!(read_visible_sections(&[], &MOST_WANTED_VISIBLE), Err(Error::NoVisibleSections)));
    let mut data = sample();
    // Cut the last drivable record short: the outer sizes stay, so shrink the chunk itself.
    let manager_len = data.len();
    data.truncate(manager_len - 6);
    let fix = |data: &mut Vec<u8>, at: usize, by: u32| {
        let size = u32::from_le_bytes(data[at..at + 4].try_into().unwrap()) - by;
        data[at..at + 4].copy_from_slice(&size.to_le_bytes());
    };
    fix(&mut data, 4, 6);
    let drivable_header = manager_len - (4 * 0x14 + 2 * (12 + 4 + 2 + 2)) - 8;
    fix(&mut data, drivable_header + 4, 6);
    assert!(matches!(read_visible_sections(&data, &MOST_WANTED_VISIBLE), Err(Error::Truncated { .. })));
}

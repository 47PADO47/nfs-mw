//! Decoding the `VisibleSectionManager` chunk (`docs/formats/maps.md`).

use blackbox_chunk::ids;

use super::{Boundary, DrivableSection, LoadingSection, VisibleSections};
use crate::layout::VisibleLayout;
use crate::{Error, Result};

/// Read the zones and visible lists from the track metadata file (e.g. `TRACKS/L2RA.BUN`).
pub fn read_visible_sections(meta: &[u8], layout: &VisibleLayout) -> Result<VisibleSections> {
    let manager = blackbox_chunk::find(meta, ids::VISIBLE_SECTION_MANAGER).ok_or(Error::NoVisibleSections)?;
    let table = |id: u32| manager.child(id).map(|c| c.payload).ok_or(Error::NoVisibleSections);

    let info = Bytes { data: table(ids::VISIBLE_SECTION_MANAGER_INFO)?, table: "VisibleSectionManagerInfo" };
    let l = &layout.info;
    let count = usize::try_from(info.i32(l.region_count)?).unwrap_or(0).min(l.region_max);
    let region = (0..count).map(|i| info.i16(l.region_list + 2 * i)).collect::<Result<_>>()?;
    let lod_offset = i16::try_from(info.i32(l.lod_offset)?).map_err(|_| info.truncated(l.lod_offset))?;

    Ok(VisibleSections {
        lod_offset,
        region,
        boundaries: read_boundaries(table(ids::VISIBLE_SECTION_BOUNDARIES)?, layout)?,
        drivable: read_drivable(table(ids::DRIVABLE_SCENERY_SECTIONS)?, layout)?,
        loading: read_loading(table(ids::LOADING_SECTIONS)?, layout)?,
    })
}

fn read_boundaries(data: &[u8], layout: &VisibleLayout) -> Result<Vec<Boundary>> {
    let l = &layout.boundary;
    let mut out = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let r = Bytes { data: data.get(at..).unwrap_or(&[]), table: "VisibleSectionBoundaries" };
        let num_points = usize::from(r.u8(l.num_points)?);
        let len = l.points + l.point_len * num_points;
        let r = Bytes { data: r.data.get(..len).ok_or(Error::Truncated { table: r.table, offset: at })?, ..r };
        out.push(Boundary {
            section: r.i16(l.section)?,
            panorama: r.u8(l.panorama)? != 0,
            bbox_min: r.vec2(l.bbox_min)?,
            bbox_max: r.vec2(l.bbox_max)?,
            centre: r.vec2(l.centre)?,
            points: (0..num_points).map(|i| r.vec2(l.points + l.point_len * i)).collect::<Result<_>>()?,
        });
        at += len;
    }
    Ok(out)
}

fn read_drivable(data: &[u8], layout: &VisibleLayout) -> Result<Vec<DrivableSection>> {
    let l = &layout.drivable;
    let mut out = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let r = Bytes { data: data.get(at..).unwrap_or(&[]), table: "DrivableScenerySections" };
        let max = usize::from(r.u8(l.max_visible)?);
        let count = usize::try_from(r.i16(l.num_visible)?).unwrap_or(0).min(max);
        out.push(DrivableSection {
            section: r.i16(l.section)?,
            visible: (0..count).map(|i| r.i16(l.list + 2 * i)).collect::<Result<_>>()?,
        });
        let len = l.list + 2 * max + l.trailing;
        if at + len > data.len() {
            return Err(Error::Truncated { table: r.table, offset: at });
        }
        at += len;
    }
    Ok(out)
}

fn read_loading(data: &[u8], layout: &VisibleLayout) -> Result<Vec<LoadingSection>> {
    let l = &layout.loading;
    data.chunks_exact(l.len)
        .map(|record| {
            let r = Bytes { data: record, table: "LoadingSections" };
            let list = |count_at: usize, list_at: usize, max: usize| -> Result<Vec<i16>> {
                let count = usize::try_from(r.i16(count_at)?).unwrap_or(0).min(max);
                (0..count).map(|i| r.i16(list_at + 2 * i)).collect()
            };
            let name = &record[l.name..l.name + l.name_len];
            let name_len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
            Ok(LoadingSection {
                name: String::from_utf8_lossy(&name[..name_len]).into_owned(),
                default: r.u8(l.default_flag)? != 0,
                drivable: list(l.num_drivable, l.drivable, l.drivable_max)?,
                extra: list(l.num_extra, l.extra, l.extra_max)?,
            })
        })
        .collect()
}

/// Bounds-checked little-endian reads from one record.
#[derive(Clone, Copy)]
struct Bytes<'a> {
    data: &'a [u8],
    table: &'static str,
}

impl Bytes<'_> {
    fn truncated(&self, offset: usize) -> Error {
        Error::Truncated { table: self.table, offset }
    }

    fn array<const N: usize>(&self, at: usize) -> Result<[u8; N]> {
        self.data.get(at..at + N).and_then(|b| b.try_into().ok()).ok_or_else(|| self.truncated(at))
    }

    fn u8(&self, at: usize) -> Result<u8> {
        Ok(self.array::<1>(at)?[0])
    }

    fn i16(&self, at: usize) -> Result<i16> {
        self.array(at).map(i16::from_le_bytes)
    }

    fn i32(&self, at: usize) -> Result<i32> {
        self.array(at).map(i32::from_le_bytes)
    }

    fn vec2(&self, at: usize) -> Result<[f32; 2]> {
        Ok([f32::from_le_bytes(self.array(at)?), f32::from_le_bytes(self.array(at + 4)?)])
    }
}

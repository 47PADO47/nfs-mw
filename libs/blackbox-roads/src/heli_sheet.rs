//! The helicopter's height sheet: the lowest height a helicopter may fly at over each drivable section, as triangles
//! in the map plane. Format and lookup: `docs/formats/pursuit-data.md` section 1 (checked against `STREAML2RA.BUN` and
//! the decompilation). A point of the map plane is `(z, -x)` of the physics (y-up) position, as in
//! [`crate::to_zone_space`]. Triangle corners are stored in quarter metres in the plane and heights in sixteenths of a
//! metre; the lookup returns the plane's height above the point.

use glam::{Vec2, Vec3};
use std::collections::HashMap;

/// Bytes of one triangle in the chunk: nine `i16` values.
const TRIANGLE_LEN: usize = 18;
/// Header bytes after the fill: two list links (4 each), the section number, the polygon count, the endian flag and
/// the table pointer (4 each).
const HEADER_AFTER_FILL: usize = 24;
/// Grid cell size in metres for the lookup index.
const CELL: f32 = 32.0;

/// One triangle: its corners in the map plane (metres) and the height of each corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Triangle {
    pub plane: [Vec2; 3],
    pub height: [f32; 3],
}

/// One sheet surface, in map coordinates `(plane.x, plane.y, height)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surface {
    pub height: f32,
    pub normal: Vec3,
}

impl Triangle {
    /// The height of this triangle's plane above `p`, or `None` when `p` is outside the triangle.
    fn height_at(&self, p: Vec2) -> Option<f32> {
        let [a, b, c] = self.plane;
        let det = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
        if det.abs() < 1e-9 {
            return None;
        }
        let l1 = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / det;
        let l2 = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / det;
        let l3 = 1.0 - l1 - l2;
        let eps = -1e-5;
        if l1 < eps || l2 < eps || l3 < eps {
            return None;
        }
        Some(l1 * self.height[0] + l2 * self.height[1] + l3 * self.height[2])
    }
}

/// The sheet of a track: its triangles and a grid over them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeliSheet {
    triangles: Vec<Triangle>,
    /// Triangle indices per grid cell.
    cells: HashMap<(i32, i32), Vec<u32>>,
}

impl HeliSheet {
    /// Builds the sheet from triangles, indexing each in every grid cell its bounds touch.
    pub fn new(triangles: Vec<Triangle>) -> Self {
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (i, t) in triangles.iter().enumerate() {
            let lo = t.plane[0].min(t.plane[1]).min(t.plane[2]);
            let hi = t.plane[0].max(t.plane[1]).max(t.plane[2]);
            for x in cell(lo.x)..=cell(hi.x) {
                for y in cell(lo.y)..=cell(hi.y) {
                    cells.entry((x, y)).or_default().push(i as u32);
                }
            }
        }
        Self { triangles, cells }
    }

    /// Parses one `HeliSheet` chunk payload (one drivable section). Returns the section number and its triangles.
    /// The fill of `0x11` bytes at the start is measured, not assumed; a payload whose size does not match its polygon
    /// count is refused.
    pub fn parse_section(payload: &[u8]) -> Option<(u32, Vec<Triangle>)> {
        let fill = payload.iter().take_while(|&&b| b == 0x11).count().min(12);
        let word = |o: usize| -> Option<u32> { Some(u32::from_le_bytes(payload.get(o..o + 4)?.try_into().ok()?)) };
        let section = word(fill + 8)?;
        let polys = word(fill + 12)? as usize;
        let body = fill + HEADER_AFTER_FILL;
        let need = body + TRIANGLE_LEN * polys;
        if payload.len() < need || payload.len() > need + 2 {
            return None;
        }
        let short = |o: usize| -> Option<i16> { Some(i16::from_le_bytes(payload.get(o..o + 2)?.try_into().ok()?)) };
        let mut triangles = Vec::with_capacity(polys);
        for n in 0..polys {
            let base = body + TRIANGLE_LEN * n;
            let x = [short(base)?, short(base + 2)?, short(base + 4)?];
            let y = [short(base + 6)?, short(base + 8)?, short(base + 10)?];
            let z = [short(base + 12)?, short(base + 14)?, short(base + 16)?];
            let plane = [0, 1, 2].map(|i| Vec2::new(x[i] as f32 / 4.0, y[i] as f32 / 4.0));
            let height = [0, 1, 2].map(|i| z[i] as f32 / 16.0);
            triangles.push(Triangle { plane, height });
        }
        Some((section, triangles))
    }

    /// The sheet's height above the map-plane point `p`, if a triangle covers it.
    pub fn elevation(&self, p: Vec2) -> Option<f32> {
        let candidates = self.cells.get(&(cell(p.x), cell(p.y)))?;
        candidates.iter().find_map(|&i| self.triangles[i as usize].height_at(p))
    }

    /// Height and upward normal of the same first triangle used by [`Self::elevation`].
    pub fn surface(&self, p: Vec2) -> Option<Surface> {
        let candidates = self.cells.get(&(cell(p.x), cell(p.y)))?;
        candidates.iter().find_map(|&i| {
            let triangle = &self.triangles[i as usize];
            let height = triangle.height_at(p)?;
            let points = [0, 1, 2].map(|n| triangle.plane[n].extend(triangle.height[n]));
            let normal = (points[1] - points[0]).cross(points[2] - points[0]).try_normalize()?;
            let normal = normal * normal.z.signum();
            Some(Surface { height, normal })
        })
    }

    /// How many triangles the sheet has.
    pub fn len(&self) -> usize {
        self.triangles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }
}

fn cell(v: f32) -> i32 {
    (v / CELL).floor() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A section payload: the fill, the header and one triangle per entry, with plane words in quarter metres and
    /// height words in sixteenths.
    fn payload(section: u32, tris: &[([i16; 3], [i16; 3], [i16; 3])], fill: usize) -> Vec<u8> {
        let mut out = vec![0x11; fill];
        out.extend_from_slice(&[11, 0, 0, 0, 11, 0, 0, 0]);
        out.extend_from_slice(&section.to_le_bytes());
        out.extend_from_slice(&(tris.len() as u32).to_le_bytes());
        out.extend_from_slice(&[0; 8]);
        for (x, y, z) in tris {
            for v in [x, y, z] {
                for w in v {
                    out.extend_from_slice(&w.to_le_bytes());
                }
            }
        }
        if tris.len() % 2 == 1 {
            out.extend_from_slice(&[0, 0]);
        }
        out
    }

    /// A right triangle in the plane with corners (0,0), (40,0), (0,40) metres and heights 10, 10, 20 metres.
    fn corner() -> ([i16; 3], [i16; 3], [i16; 3]) {
        ([0, 160, 0], [0, 0, 160], [160, 160, 320])
    }

    #[test]
    fn parses_the_section_number_and_its_triangles() {
        let bytes = payload(2030, &[corner()], 4);
        let (section, tris) = HeliSheet::parse_section(&bytes).expect("parses");
        assert_eq!(section, 2030);
        assert_eq!(tris.len(), 1);
        assert_eq!(tris[0].plane[1], Vec2::new(40.0, 0.0));
        assert_eq!(tris[0].height, [10.0, 10.0, 20.0]);
    }

    #[test]
    fn the_fill_length_is_measured_from_the_leading_bytes() {
        for fill in [0usize, 4, 8, 12] {
            let bytes = payload(101, &[corner()], fill);
            assert!(HeliSheet::parse_section(&bytes).is_some(), "fill {fill}");
        }
    }

    #[test]
    fn a_size_that_does_not_match_the_count_is_refused() {
        let mut bytes = payload(101, &[corner()], 0);
        bytes.truncate(bytes.len() - 4);
        assert!(HeliSheet::parse_section(&bytes).is_none());
    }

    #[test]
    fn the_elevation_is_the_plane_height_inside_the_triangle() {
        let (_, tris) = HeliSheet::parse_section(&payload(101, &[corner()], 0)).expect("parses");
        let sheet = HeliSheet::new(tris);
        // The corners are (0, 0) at 10 m, (40, 0) at 10 m and (0, 40) at 20 m. The midpoint of the edge from (0, 0) to
        // (0, 40) is (0, 20), where the plane is halfway between 10 m and 20 m: 15 m.
        let h = sheet.elevation(Vec2::new(0.0, 20.0)).expect("covered");
        assert!((h - 15.0).abs() < 1e-3, "{h}");
        assert_eq!(sheet.elevation(Vec2::new(30.0, 30.0)), None, "outside the triangle");
    }

    #[test]
    fn points_in_other_grid_cells_find_their_triangle() {
        let far = ([4000, 4160, 4000], [4000, 4000, 4160], [4160, 4160, 4320]);
        let (_, tris) = HeliSheet::parse_section(&payload(101, &[corner(), far], 0)).expect("parses");
        let sheet = HeliSheet::new(tris);
        assert_eq!(sheet.len(), 2);
        assert!(sheet.elevation(Vec2::new(1000.0, 1010.0)).is_some());
        assert!(sheet.elevation(Vec2::new(0.0, 20.0)).is_some());
    }

    #[test]
    fn surface_is_upward_for_either_winding_and_uses_the_elevation_triangle() {
        let triangle =
            Triangle { plane: [Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(0.0, 10.0)], height: [0.0, 10.0, 0.0] };
        for winding in [[0, 1, 2], [2, 1, 0]] {
            let triangle =
                Triangle { plane: winding.map(|i| triangle.plane[i]), height: winding.map(|i| triangle.height[i]) };
            let sheet = HeliSheet::new(vec![triangle]);
            let at = Vec2::new(2.0, 3.0);
            let surface = sheet.surface(at).unwrap();
            assert_eq!(Some(surface.height), sheet.elevation(at));
            assert!((surface.normal - Vec3::new(-1.0, 0.0, 1.0).normalize()).length() < 1e-5);
        }
    }
}

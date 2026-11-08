//! The unit's collision (`PolySoupList`, 0x60, `docs/formats/chameleon-world.md#collision-polysouplist-0x60`) as the triangles of
//! the game's collision pack: chunks of up to 255 vertices stored as 16-bit offsets in 500 m cells, faces that are triangles or
//! quads, and a flag word per face that this work reads only for the surface under the face.

use crate::space;
use crate::unit::Unit;
use anyhow::{Result, ensure};
use blackbox_attrib::vlt_hash;
use blackbox_collision::SoupTriangle;

/// Resource type id of a `PolySoupList`.
pub const POLY_SOUP_LIST: u32 = 0x60;
/// The header: the offset of the table of chunk offsets and the chunk count.
const TABLE_AT: usize = 0x20;
const COUNT_AT: usize = 0x28;
/// A chunk: the offsets of its faces and vertices, the three signed cells and the two counts.
const CELLS_AT: usize = 0x0A;
const FACE_COUNT_AT: usize = 0x0E;
const VERTEX_COUNT_AT: usize = 0x0F;
/// The metres a 16-bit coordinate step is worth: 500 m over 32768, and the size of a cell.
const STEP: f32 = 500.0 / 32768.0;
const CELL: f32 = 500.0;
const FACE_LEN: usize = 12;
const VERTEX_LEN: usize = 6;
/// The face flag word's first bit: set on the faces of roads and other ground a car drives on (**[unconfirmed]**, read off the
/// flat faces of the units: 99 % of the faces with it are flat, and they make the streets and quays).
const DRIVE_BIT: u32 = 1;

/// One triangle of a unit's collision in engine space, with its flag word.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Face {
    pub points: [[f32; 3]; 3],
    pub flags: u32,
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

/// The triangles of a poly soup list's main block. A quad's corners are in strip order, so it is drawn 0, 1, 3, 2 and is two
/// triangles.
pub fn decode(main: &[u8]) -> Result<Vec<Face>> {
    let (Some(table), Some(count)) = (u32_at(main, TABLE_AT), u32_at(main, COUNT_AT)) else {
        anyhow::bail!("the poly soup list is too short for its header");
    };
    let (table, count) = (table as usize, count as usize);
    ensure!(table + count * 4 <= main.len(), "the poly soup list says {count} chunks and holds {} bytes", main.len());
    let mut faces = Vec::new();
    for k in 0..count {
        let Some(chunk) = u32_at(main, table + 4 * k) else { continue };
        let chunk = chunk as usize;
        let (Some(face_at), Some(vertex_at)) = (u32_at(main, chunk), u32_at(main, chunk + 4)) else { continue };
        let (Some(&face_count), Some(&vertex_count)) =
            (main.get(chunk + FACE_COUNT_AT), main.get(chunk + VERTEX_COUNT_AT))
        else {
            continue;
        };
        let Some(cells) = main.get(chunk + CELLS_AT..chunk + CELLS_AT + 3) else { continue };
        let cell = [0, 1, 2].map(|axis| f32::from(cells[axis] as i8) * CELL);
        let mut vertices = Vec::with_capacity(usize::from(vertex_count));
        for n in 0..usize::from(vertex_count) {
            let at = vertex_at as usize + n * VERTEX_LEN;
            let q = [0, 1, 2].map(|axis| u16_at(main, at + 2 * axis));
            let [Some(x), Some(y), Some(z)] = q else { break };
            vertices.push([
                f32::from(x) * STEP + cell[0],
                f32::from(y) * STEP + cell[1],
                f32::from(z) * STEP + cell[2],
            ]);
        }
        for n in 0..usize::from(face_count) {
            let at = face_at as usize + n * FACE_LEN;
            let (Some(flags), Some(corners)) = (u32_at(main, at), main.get(at + 4..at + 8)) else { continue };
            let corner: Vec<usize> =
                corners.iter().filter(|&&index| index != 0xFF).map(|&index| usize::from(index)).collect();
            let point = |slot: usize| corner.get(slot).and_then(|&index| vertices.get(index)).copied();
            match corner.len() {
                3 => {
                    if let (Some(a), Some(b), Some(c)) = (point(0), point(1), point(2)) {
                        faces.push(Face { points: [a, b, c], flags });
                    }
                }
                4 => {
                    if let (Some(a), Some(b), Some(c), Some(d)) = (point(0), point(1), point(3), point(2)) {
                        faces.push(Face { points: [a, b, c], flags });
                        faces.push(Face { points: [a, c, d], flags });
                    }
                }
                _ => {}
            }
        }
    }
    Ok(faces)
}

/// The `simsurface` key of the ground under a face: asphalt where the face's drive bit is set, grass on the other flat ground.
/// Steep faces are walls and slopes of terrain and take the grass.
pub fn surface_of(flags: u32) -> u32 {
    vlt_hash(if flags & DRIVE_BIT != 0 { "asphalt" } else { "grass" })
}

/// A face as a triangle of the game's collision: physics space, wound so that it faces up (the ground query takes the side a
/// ray comes from, and the 2012 soup's winding is not consistent), with its surface.
pub fn triangle(face: &Face) -> SoupTriangle {
    let mut points = face.points.map(space::physics);
    let (u, v) = ([0, 1, 2].map(|k| points[1][k] - points[0][k]), [0, 1, 2].map(|k| points[2][k] - points[0][k]));
    // The game's ground query takes a triangle as facing up when its corners turn this way round in x and z (`faces_up` of
    // the collision pack writer): the cross product below is negative.
    let turn = u[2] * v[0] - u[0] * v[2];
    if turn > 0.0 {
        points.swap(1, 2);
    }
    SoupTriangle { points, surface: surface_of(face.flags) }
}

/// The collision triangles of a unit, in physics space. A unit has one soup list.
pub fn ground(unit: &Unit<'_>) -> Result<Vec<SoupTriangle>> {
    let mut out = Vec::new();
    for id in unit.ids_of(POLY_SOUP_LIST) {
        let resource = unit.resource(id)?;
        out.extend(decode(&resource.blocks[0])?.iter().map(triangle));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A soup list of one chunk in cells (`cell`), with vertices given as raw 16-bit steps and faces as (flags, four indices).
    fn soup(cell: [i8; 3], vertices: &[[u16; 3]], faces: &[(u32, [u8; 4])]) -> Vec<u8> {
        let table = 0x30usize;
        let chunk = 0x40usize;
        let face_at = chunk + 0x10;
        let vertex_at = face_at + faces.len() * FACE_LEN;
        let mut out = vec![0u8; vertex_at + vertices.len() * VERTEX_LEN];
        out[TABLE_AT..TABLE_AT + 4].copy_from_slice(&(table as u32).to_le_bytes());
        out[COUNT_AT..COUNT_AT + 4].copy_from_slice(&1u32.to_le_bytes());
        out[table..table + 4].copy_from_slice(&(chunk as u32).to_le_bytes());
        out[chunk..chunk + 4].copy_from_slice(&(face_at as u32).to_le_bytes());
        out[chunk + 4..chunk + 8].copy_from_slice(&(vertex_at as u32).to_le_bytes());
        for (k, c) in cell.iter().enumerate() {
            out[chunk + CELLS_AT + k] = *c as u8;
        }
        out[chunk + FACE_COUNT_AT] = faces.len() as u8;
        out[chunk + VERTEX_COUNT_AT] = vertices.len() as u8;
        for (n, (flags, corners)) in faces.iter().enumerate() {
            let at = face_at + n * FACE_LEN;
            out[at..at + 4].copy_from_slice(&flags.to_le_bytes());
            out[at + 4..at + 8].copy_from_slice(corners);
        }
        for (n, q) in vertices.iter().enumerate() {
            for (axis, value) in q.iter().enumerate() {
                let at = vertex_at + n * VERTEX_LEN + 2 * axis;
                out[at..at + 2].copy_from_slice(&value.to_le_bytes());
            }
        }
        out
    }

    #[test]
    fn a_vertex_is_a_step_count_plus_its_cell_of_500_metres() {
        let main = soup([-3, -1, 2], &[[0, 0, 0], [32768 / 2, 0, 0], [0, 0, 16384]], &[(7, [0, 1, 2, 0xFF])]);
        let faces = decode(&main).unwrap();
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0].flags, 7);
        assert_eq!(faces[0].points[0], [-1500.0, -500.0, 1000.0]);
        assert_eq!(faces[0].points[1], [-1250.0, -500.0, 1000.0]);
        assert_eq!(faces[0].points[2], [-1500.0, -500.0, 1250.0]);
    }

    #[test]
    fn a_quad_is_two_triangles_drawn_in_strip_order() {
        // Corners 0, 1, 2, 3 in file order are drawn 0, 1, 3, 2.
        let main = soup([0, 0, 0], &[[0, 0, 0], [100, 0, 0], [0, 0, 100], [100, 0, 100]], &[(1, [0, 1, 2, 3])]);
        let faces = decode(&main).unwrap();
        assert_eq!(faces.len(), 2);
        // The polygon is vertices 0, 1, 3, 2: triangles (0, 1, 3) and (0, 3, 2).
        assert_eq!(faces[0].points[2], faces[1].points[1]);
        let far = 100.0 * STEP;
        assert_eq!(faces[0].points[2], [far, 0.0, far]);
        assert_eq!(faces[1].points[2], [0.0, 0.0, far]);
    }

    #[test]
    fn a_short_list_is_refused_and_a_cut_chunk_is_skipped() {
        assert!(decode(&[0; 8]).is_err());
        let mut main = soup([0, 0, 0], &[[0, 0, 0]], &[(1, [0, 1, 2, 0xFF])]);
        main.truncate(0x45);
        assert!(decode(&main).unwrap().is_empty());
    }

    #[test]
    fn a_triangle_becomes_physics_space_facing_up_with_its_surface() {
        // Engine corners wound clockwise from above, and counter-clockwise: both come out facing up.
        for corners in [
            [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 0.0, 10.0]],
            [[0.0, 0.0, 0.0], [0.0, 0.0, 10.0], [10.0, 0.0, 0.0]],
        ] {
            let soup = triangle(&Face { points: corners, flags: DRIVE_BIT });
            let [a, b, c] = soup.points;
            let (u, v) = ([b[0] - a[0], b[2] - a[2]], [c[0] - a[0], c[2] - a[2]]);
            assert!(u[1] * v[0] - u[0] * v[1] < 0.0, "{soup:?}");
            assert_eq!(a[0], -corners[0][0], "physics x is minus engine x");
            assert_eq!(soup.surface, vlt_hash("asphalt"));
        }
        assert_eq!(surface_of(0x40), vlt_hash("grass"));
        assert_eq!(surface_of(0x0A104001), vlt_hash("asphalt"));
    }
}

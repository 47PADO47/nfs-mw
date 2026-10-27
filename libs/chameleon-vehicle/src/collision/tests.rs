use std::path::Path;

use chameleon_bundle::{Bundle, Pool};

use super::*;
use crate::find_spec;

/// One chunk to build: its axis flags, the stored `u16` triples and the triangles.
struct Chunk {
    flags: [u8; 4],
    vertices: Vec<[u16; 3]>,
    triangles: Vec<[u8; 3]>,
}

/// A list block laid out the way the files are: header, table of chunk offsets, then each chunk's header, vertices
/// and triangles.
fn block(chunks: &[Chunk]) -> Vec<u8> {
    let mut out = vec![0u8; 0x30 + 4 * chunks.len()];
    out[TABLE_AT..TABLE_AT + 4].copy_from_slice(&0x30u32.to_le_bytes());
    out[COUNT_AT..COUNT_AT + 4].copy_from_slice(&(chunks.len() as u32).to_le_bytes());
    for (n, chunk) in chunks.iter().enumerate() {
        let at = out.len();
        out[0x30 + 4 * n..0x34 + 4 * n].copy_from_slice(&(at as u32).to_le_bytes());
        let vertex_table = at + 16;
        let triangle_table = vertex_table + chunk.vertices.len() * VERTEX_LEN;
        let mut header = vec![0u8; 16];
        header[..4].copy_from_slice(&(triangle_table as u32).to_le_bytes());
        header[4..8].copy_from_slice(&(vertex_table as u32).to_le_bytes());
        header[FLAGS_AT..FLAGS_AT + 4].copy_from_slice(&chunk.flags);
        header[TRIANGLE_COUNT_AT] = chunk.triangles.len() as u8;
        header[VERTEX_COUNT_AT] = chunk.vertices.len() as u8;
        out.extend_from_slice(&header);
        for v in &chunk.vertices {
            v.iter().for_each(|c| out.extend_from_slice(&c.to_le_bytes()));
        }
        for t in &chunk.triangles {
            out.extend_from_slice(&[0, 0, 0, 0, t[0], t[1], t[2], 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        }
    }
    out
}

fn steps(metres: f32) -> u16 {
    (metres / STEP).round() as u16
}

#[test]
fn a_chunk_decodes_by_axis_flags() {
    // x and z centred (half a range below zero), y from zero: the layout the body chunks use.
    let chunk = Chunk {
        flags: [0xFF, 0, 0xFF, 0],
        vertices: vec![
            [32768 - steps(0.9), steps(0.17), 32768 - steps(2.2)],
            [32768 + steps(0.9), steps(1.3), 32768 + steps(2.2)],
            [32768, steps(0.5), 32768],
        ],
        triangles: vec![[0, 1, 2]],
    };
    let soup = PolySoup::parse(&block(&[chunk])).unwrap().unwrap();
    assert_eq!(soup.triangles, vec![[0, 1, 2]]);
    let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.02);
    assert!(near(soup.vertices[0], [-0.9, 0.17, -2.2]), "{:?}", soup.vertices[0]);
    assert!(near(soup.vertices[1], [0.9, 1.3, 2.2]));
    assert!(near(soup.vertices[2], [0.0, 0.5, 0.0]));
    let (min, max) = soup.extents();
    assert!(near(min, [-0.9, 0.17, -2.2]) && near(max, [0.9, 1.3, 2.2]));
}

#[test]
fn a_chunk_that_marks_no_axis_centred_reads_from_zero() {
    // The nose chunk of a car: z from zero as well.
    let chunk = Chunk {
        flags: [0xFF, 0, 0, 0],
        vertices: vec![
            [32768, steps(0.2), steps(1.5)],
            [32768 + steps(0.5), steps(0.2), steps(2.2)],
            [32768, steps(0.9), steps(2.0)],
        ],
        triangles: vec![[0, 1, 2]],
    };
    let soup = PolySoup::parse(&block(&[chunk])).unwrap().unwrap();
    assert!((soup.vertices[0][2] - 1.5).abs() < 0.02 && (soup.vertices[1][0] - 0.5).abs() < 0.02);
}

#[test]
fn a_vertex_no_triangle_uses_is_dropped() {
    let chunk = Chunk {
        flags: [0xFF, 0, 0xFF, 0],
        vertices: vec![[32768, 100, 0], [32768, 200, 32768], [32768, 300, 32768], [32768, 400, 32768]],
        triangles: vec![[0, 2, 3]],
    };
    let soup = PolySoup::parse(&block(&[chunk])).unwrap().unwrap();
    assert_eq!(soup.vertices.len(), 3, "vertex 1 is not used");
    // Vertex 1 is the one nobody uses; the others keep their order.
    assert_eq!(soup.triangles, vec![[0, 1, 2]]);
}

#[test]
fn chunks_share_one_vertex_list() {
    let chunk = |y: u16| Chunk {
        flags: [0xFF, 0, 0xFF, 0],
        vertices: vec![[32768, y, 32768], [32768 + 10, y, 32768], [32768, y, 32768 + 10]],
        triangles: vec![[0, 1, 2]],
    };
    let soup = PolySoup::parse(&block(&[chunk(10), chunk(20)])).unwrap().unwrap();
    assert_eq!((soup.vertices.len(), soup.triangles.len()), (6, 2));
    assert_eq!(soup.triangles[1], [3, 4, 5]);
}

#[test]
fn a_list_without_chunks_is_none() {
    assert_eq!(PolySoup::parse(&block(&[])).unwrap(), None);
}

#[test]
fn broken_lists_are_errors() {
    assert!(PolySoup::parse(&[0u8; 8]).is_err(), "no header");
    let bad_index = Chunk { flags: [0; 4], vertices: vec![[0; 3]; 3], triangles: vec![[0, 1, 3]] };
    assert!(PolySoup::parse(&block(&[bad_index])).is_err(), "index past the vertices");
    let mut truncated = block(&[Chunk { flags: [0; 4], vertices: vec![[0; 3]; 3], triangles: vec![[0, 1, 2]] }]);
    truncated.truncate(truncated.len() - 6);
    assert!(PolySoup::parse(&truncated).is_err(), "cut inside the triangles");
    let mut many = block(&[]);
    many[COUNT_AT..COUNT_AT + 4].copy_from_slice(&5000u32.to_le_bytes());
    assert!(PolySoup::parse(&many).is_err(), "absurd chunk count");
}

#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR)"]
fn every_car_has_a_body_sized_collision_mesh_but_one() {
    let Some(dir) = std::env::var_os("CHAMELEON_GAME_DIR") else { return };
    let dir = Path::new(&dir);
    let (mut with, mut without) = (0, 0);
    for entry in std::fs::read_dir(dir.join("VEHICLES")).unwrap().flatten() {
        let path = entry.path();
        let file = path.file_name().unwrap().to_string_lossy().to_uppercase();
        if !(file.starts_with("VEH_") && file.ends_with("_HI.BNDL")) {
            continue;
        }
        let data = std::fs::read(&path).unwrap();
        let bundle = Bundle::parse(&data).unwrap();
        let spec = find_spec(&bundle).unwrap();
        let mut pool = Pool::new();
        pool.add(bundle.clone());
        let Some(soup) = load_poly_soup(&pool, spec).unwrap() else {
            without += 1;
            continue;
        };
        let (min, max) = soup.extents();
        assert!(soup.triangles.len() > 100 && soup.triangles.len() < 400, "{file}: {} triangles", soup.triangles.len());
        assert!(max[0] - min[0] > 1.5 && max[0] - min[0] < 2.4, "{file}: width {:?} {:?}", min, max);
        assert!(max[1] - min[1] > 0.8 && max[1] < 2.0 && min[1] >= -0.01, "{file}: height {:?} {:?}", min, max);
        assert!(max[2] - min[2] > 3.0 && max[2] - min[2] < 6.0, "{file}: length {:?} {:?}", min, max);
        with += 1;
    }
    println!("{with} cars with a collision mesh, {without} without");
    assert!(with > 50 && without <= 2);
}

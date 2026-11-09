//! Tests of the gear importer on authored data: meshes and pictures made here, never RIDE 6's.

use std::fs;
use std::path::PathBuf;

use blackbox_skeleton::Pack;
use serde_json::{Value, json};

use super::mesh::{self, Role};
use super::texture::{self, BC1, Mip, Texture};
use super::{DEFAULT_MAX_TEXTURE, import};

fn put_string(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(&(text.len() as u16).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
}

fn put_floats(out: &mut Vec<u8>, values: &[f32]) {
    values.iter().for_each(|v| out.extend_from_slice(&v.to_le_bytes()));
}

/// One vertex: its place and the `(bone, weight)` pairs it follows.
type Vertex<'a> = ([f32; 3], &'a [(u16, f32)]);

/// An `R6RM` file of one level of detail (`docs/formats/ride6-rider.md`). Sections are `(material, first index, faces)`.
fn r6rm(
    bones: &[(&str, i32)],
    materials: &[(&str, &str)],
    vertices: &[Vertex],
    indices: &[u32],
    sections: &[(i32, u32, u32)],
) -> Vec<u8> {
    let mut out = b"R6RM".to_vec();
    out.extend_from_slice(&1u32.to_le_bytes());
    put_string(&mut out, "test_mesh");
    out.extend_from_slice(&(bones.len() as u32).to_le_bytes());
    for (name, parent) in bones {
        put_string(&mut out, name);
        out.extend_from_slice(&parent.to_le_bytes());
        put_floats(&mut out, &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
    }
    out.extend_from_slice(&(materials.len() as u32).to_le_bytes());
    for (slot, asset) in materials {
        put_string(&mut out, slot);
        put_string(&mut out, asset);
    }
    out.extend_from_slice(&1u32.to_le_bytes());
    put_floats(&mut out, &[1.0]);
    out.extend_from_slice(&(vertices.len() as u32).to_le_bytes());
    out.extend_from_slice(&(indices.len() as u32).to_le_bytes());
    out.extend_from_slice(&(sections.len() as u32).to_le_bytes());
    for (material, first, faces) in sections {
        out.extend_from_slice(&material.to_le_bytes());
        out.extend_from_slice(&first.to_le_bytes());
        out.extend_from_slice(&faces.to_le_bytes());
    }
    for (position, influences) in vertices {
        put_floats(&mut out, position);
        put_floats(&mut out, &[0.0, 0.0, 1.0, 0.5, 0.5]);
        out.push(influences.len() as u8);
        for (bone, weight) in *influences {
            out.extend_from_slice(&bone.to_le_bytes());
            out.extend_from_slice(&weight.to_le_bytes());
        }
    }
    indices.iter().for_each(|i| out.extend_from_slice(&i.to_le_bytes()));
    out
}

/// A body of six vertices: three on the root, three on the left hand; a triangle on each and one across the wrist.
fn body_file() -> Vec<u8> {
    let root: &[(u16, f32)] = &[(0, 1.0)];
    let hand: &[(u16, f32)] = &[(1, 1.0)];
    let vertices: Vec<Vertex> = vec![
        ([0.0, 0.0, 0.0], root),
        ([1.0, 0.0, 0.0], root),
        ([0.0, 1.0, 0.0], root),
        ([5.0, 0.0, 0.0], hand),
        ([6.0, 0.0, 0.0], hand),
        ([5.0, 1.0, 0.0], hand),
    ];
    r6rm(
        &[("root", -1), ("hand_l", 0)],
        &[("suit_mat", "BASE_suit")],
        &vertices,
        &[0, 1, 2, 3, 4, 5, 2, 3, 4],
        &[(0, 0, 3)],
    )
}

fn helmet_file() -> Vec<u8> {
    let on: &[(u16, f32)] = &[(0, 1.0)];
    let vertices: Vec<Vertex> = vec![([0.0, 0.0, 0.0], on), ([1.0, 0.0, 0.0], on), ([0.0, 1.0, 0.0], on)];
    r6rm(
        &[("helmet", -1)],
        &[("helmet_mat", "mat_helmet000"), ("goggles_mat", "mat_goggle001"), ("Default_Material", "WorldGridMaterial")],
        &vertices,
        &[0, 1, 2, 0, 1, 2, 0, 1, 2],
        &[(0, 0, 1), (1, 3, 1), (2, 6, 1)],
    )
}

#[test]
fn the_hand_triangles_of_a_body_are_a_section_of_their_own() {
    let pack = Pack::parse(&mesh::pack_bytes(&body_file(), Role::Body).unwrap()).unwrap();
    let body = &pack.meshes[0];
    assert_eq!(body.name, "body");
    assert_eq!(body.materials, [mesh::MAIN, mesh::HANDS]);
    let lod = &body.lods[0];
    assert_eq!(pack.skeleton.len(), 2, "the pack's skeleton is the mesh's own bones");
    assert_eq!(lod.sections.len(), 2);
    // The body triangle and the one across the wrist stay; the hand triangle goes last.
    assert_eq!((lod.sections[0].material, lod.sections[0].first_index, lod.sections[0].index_count), (0, 0, 6));
    assert_eq!((lod.sections[1].material, lod.sections[1].first_index, lod.sections[1].index_count), (1, 6, 3));
    // The converter re-winds every triangle for the mirrored axes: (3, 4, 5) is (3, 5, 4).
    assert_eq!(&lod.indices[6..], &[3u32, 5, 4]);
    assert_eq!(lod.indices.len(), 9);
}

#[test]
fn only_a_body_has_hands() {
    let pack = Pack::parse(&mesh::pack_bytes(&body_file(), Role::Boots).unwrap()).unwrap();
    assert_eq!(pack.meshes[0].materials, [mesh::MAIN]);
    assert_eq!(pack.meshes[0].lods[0].sections.len(), 1);
}

#[test]
fn a_helmet_keeps_its_shell_greys_its_goggle_band_and_loses_the_placeholder() {
    let pack = Pack::parse(&mesh::pack_bytes(&helmet_file(), Role::Helmet).unwrap()).unwrap();
    let helmet = &pack.meshes[0];
    assert_eq!(helmet.name, crate::camera::HIDDEN_MESHES[0], "the first-person camera leaves it out by this name");
    assert_eq!(helmet.materials, [mesh::MAIN, mesh::GOGGLES]);
    let lod = &helmet.lods[0];
    assert_eq!(lod.sections.len(), 2);
    assert_eq!(lod.indices.len(), 6, "the placeholder's triangle is gone");
    let visor = Pack::parse(&mesh::pack_bytes(&helmet_file(), Role::Visor).unwrap()).unwrap();
    assert_eq!(visor.meshes[0].name, crate::camera::HIDDEN_MESHES[1]);
}

#[test]
fn a_mesh_with_nothing_to_draw_is_refused() {
    let on: &[(u16, f32)] = &[(0, 1.0)];
    let vertices: Vec<Vertex> = vec![([0.0; 3], on), ([1.0, 0.0, 0.0], on), ([0.0, 1.0, 0.0], on)];
    let only_stray = r6rm(&[("helmet", -1)], &[("lambert1", "None")], &vertices, &[0, 1, 2], &[(0, 0, 1)]);
    assert!(mesh::pack_bytes(&only_stray, Role::Helmet).is_err());
    let past_the_end = r6rm(&[("helmet", -1)], &[("helmet_mat", "m")], &vertices, &[0, 1, 9], &[(0, 0, 1)]);
    assert!(mesh::pack_bytes(&past_the_end, Role::Helmet).is_err());
}

/// A picture of BC1 levels `sides` pixels square.
fn picture(sides: &[u32]) -> Texture {
    let mips = sides
        .iter()
        .map(|&side| {
            let blocks = (side / 4) * (side / 4);
            Mip { width: side, height: side, data: vec![side as u8; (blocks * 8) as usize] }
        })
        .collect();
    Texture { format: BC1, srgb: true, mips }
}

#[test]
fn a_picture_round_trips_and_is_checked() {
    let original = picture(&[16, 8, 4]);
    let bytes = texture::to_bytes(&original);
    assert_eq!(texture::parse(&bytes).unwrap(), original);
    assert!(texture::parse(&bytes[..bytes.len() - 1]).is_err(), "cut short");
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(texture::parse(&longer).is_err(), "trailing bytes");
    let mut wrong = original.clone();
    wrong.mips[0].data.pop();
    assert!(texture::parse(&texture::to_bytes(&wrong)).is_err(), "a level of the wrong size");
    assert!(texture::parse(b"R6TX").is_err());
}

#[test]
fn the_levels_above_the_cap_are_dropped_and_the_smallest_stays_when_all_are_above() {
    let mut texture = picture(&[16, 8, 4]);
    texture::limit(&mut texture, 8);
    assert_eq!(texture.mips.iter().map(|m| m.width).collect::<Vec<_>>(), [8, 4]);
    texture::limit(&mut texture, 2);
    assert_eq!(texture.mips.iter().map(|m| m.width).collect::<Vec<_>>(), [4]);
    let mut wide =
        Texture { format: BC1, srgb: false, mips: vec![Mip { width: 16, height: 8, data: vec![0; 4 * 2 * 8] }] };
    texture::limit(&mut wide, 8);
    assert_eq!(wide.mips.len(), 1, "a level is judged by its longer side, and the last one is never dropped");
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ride6-gear-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_catalogue_becomes_the_runtime_files_and_an_item_without_its_mesh_is_left_out() {
    let catalogue = temp("catalogue");
    fs::create_dir_all(catalogue.join("meshes")).unwrap();
    fs::create_dir_all(catalogue.join("tex")).unwrap();
    fs::write(catalogue.join("meshes/body.r6rm"), body_file()).unwrap();
    fs::write(catalogue.join("meshes/helmet.r6rm"), helmet_file()).unwrap();
    fs::write(catalogue.join("tex/pic.r6tx"), texture::to_bytes(&picture(&[16, 8, 4]))).unwrap();
    let text = json!({
        "version": 1,
        "bodies": [{ "id": "suit001_male", "gender": "male", "shape": 1, "mesh": "body" }],
        "helmets": [
            { "id": "HL_A", "name": "A one", "brand": "A", "mesh": "helmet", "color": "pic", "visor": { "mesh": "helmet", "color": "pic" } },
            { "id": "HL_B", "name": "B two", "brand": "B", "mesh": "lost", "color": "pic" }
        ],
        "suits": [{ "id": "SU_A", "name": "Suit", "brand": "S", "color": "pic" }],
        "gloves": [{ "id": "GL_A", "name": "Glove", "brand": "G", "rgb": [10, 20, 30] }],
        "boots": [],
        "meshes": { "body": { "file": "meshes/body.r6rm" }, "helmet": { "file": "meshes/helmet.r6rm" }, "lost": { "file": "meshes/lost.r6rm" } },
        "textures": { "pic": { "file": "tex/pic.r6tx" } }
    });
    fs::write(catalogue.join("catalogue.json"), text.to_string()).unwrap();

    let output = temp("output");
    let summary = import(&catalogue, &output, 8).unwrap();
    assert_eq!((summary.bodies, summary.helmets, summary.suits, summary.gloves, summary.boots), (1, 1, 1, 1, 0));
    assert_eq!(summary.textures, 1);
    assert!(summary.left_out.iter().any(|line| line.contains("lost")), "{:?}", summary.left_out);
    assert!(summary.left_out.iter().any(|line| line.contains("HL_B")), "{:?}", summary.left_out);
    // The helmet mesh is used as a helmet and as the visor of the first helmet: the first role stays and the clash is named.
    assert!(summary.left_out.iter().any(|line| line.contains("used as")), "{:?}", summary.left_out);

    let gear: Value = serde_json::from_slice(&fs::read(output.join("gear.json")).unwrap()).unwrap();
    assert_eq!(gear["version"], 1);
    assert_eq!(gear["stock"]["helmet"], json!(["helmetRacing_default"]));
    assert_eq!(gear["stock"]["body"], json!(["suit001_male", "SuitRacing_Male_arms"]));
    assert_eq!(gear["bodies"][0]["name"], "Male 1");
    assert_eq!(gear["bodies"][0]["mesh"], "meshes/body.bbsk");
    assert_eq!(gear["helmets"].as_array().unwrap().len(), 1);
    assert_eq!(gear["helmets"][0]["color"], "tex/pic.r6tx");
    assert_eq!(gear["gloves"][0]["rgb"], json!([10, 20, 30]));
    assert!(output.join("meshes/body.bbsk").is_file() && output.join("meshes/helmet.bbsk").is_file());
    assert!(!output.join("meshes/lost.bbsk").exists());
    let cut = texture::parse(&fs::read(output.join("tex/pic.r6tx")).unwrap()).unwrap();
    assert_eq!(cut.mips.iter().map(|m| m.width).collect::<Vec<_>>(), [8, 4], "cut at 8 pixels");
    fs::remove_dir_all(&catalogue).unwrap();
    fs::remove_dir_all(&output).unwrap();
}

#[test]
fn a_catalogue_of_another_version_or_none_is_an_error() {
    let dir = temp("version");
    assert!(import(&dir, &dir.join("out"), DEFAULT_MAX_TEXTURE).is_err(), "no catalogue.json");
    fs::write(dir.join("catalogue.json"), r#"{ "version": 2 }"#).unwrap();
    assert!(import(&dir, &dir.join("out"), DEFAULT_MAX_TEXTURE).is_err());
    fs::remove_dir_all(&dir).unwrap();
}

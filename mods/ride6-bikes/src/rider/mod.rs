//! The RIDE 6 rider: suit, helmet and boots as skinned meshes, the rider skeleton and the riding animations,
//! converted from the user's own exports into the pack the generic runtime ([`blackbox_skeleton`]) plays.
//!
//! Input, a folder the export tool wrote (formats in `docs/formats/ride6-rider.md`):
//!
//! ```text
//! riding.r6ra         every riding animation, sampled (and any other *.r6ra: the fall and get-up animations)
//! acl/                 animations stored with ACL: <name>.acl and <name>.meta.json, decoded here ([`acl_clips`])
//! skeleton.json       the rider skeleton and its sockets
//! bike_skeleton.json  the bike's skeleton and sockets (seat, pegs, grips)
//! meshes/*.r6rm       the skinned meshes
//! textures/*.png      the base colour pictures
//! ```
//!
//! Output, in `files/RIDER/` of the mod (never committed): `rider.bbsk` (the pack), `graph.json` (the blend tree),
//! `attach.json` (seat, pegs, grips, the limbs to solve and the numbers that drive the animation), `parts.json` (meshes and
//! their pictures) and the pictures.
//!
//! Each bike's own seat, grips and pegs are not in that folder: `ride6-bike-import` writes them per car ([`contacts`]).

pub mod acl_clips;
pub mod attach;
pub mod contacts;
pub mod convert;
pub mod graph;
pub mod parts;
pub mod raw;
pub mod seat_surface;

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// What one run wrote.
#[derive(Debug)]
pub struct Summary {
    pub bones: usize,
    pub clips: usize,
    pub meshes: usize,
    pub bytes: usize,
}

/// Converts the export in `input` into `output`. `riding_style` picks the lean clips (see [`graph::motard_graph`]).
pub fn import(input: &Path, output: &Path, riding_style: u8) -> Result<Summary> {
    let read = |name: &str| fs::read(input.join(name)).with_context(|| format!("reading {name}"));
    let rider_json = raw::parse_skeleton(&read("skeleton.json")?)?;
    let bike_json = raw::parse_skeleton(&read("bike_skeleton.json")?)?;
    let (skeleton, sockets) = convert::skeleton(&rider_json)?;
    let (anims, undecoded) = acl_clips::collect(input, &rider_json)?;
    for message in &undecoded {
        eprintln!("warning: an ACL clip was left out: {message}");
    }
    let clips = convert::clips(&anims, &skeleton)?;

    let mut meshes = Vec::new();
    let mut names = Vec::new();
    let mut mesh_files: Vec<_> = fs::read_dir(input.join("meshes"))
        .context("reading meshes/")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "r6rm"))
        .collect();
    mesh_files.sort();
    for path in &mesh_files {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let raw_mesh = raw::parse_mesh(&bytes).with_context(|| path.display().to_string())?;
        meshes.push(convert::mesh(&raw_mesh, &skeleton)?);
        names.push(raw_mesh.name.clone());
    }

    let reference = clips
        .iter()
        .find(|c| c.name == attach::REFERENCE_CLIP)
        .with_context(|| format!("the animation {} is not in the export", attach::REFERENCE_CLIP))?
        .sample(&skeleton, 0.0, false);
    let attach = attach::attach_json(&skeleton, &reference, &bike_json)?;
    let pack = convert::pack(skeleton, sockets, clips, meshes);
    let graph = graph::motard_graph(&pack, riding_style)?;
    graph.validate(&pack.clips, &pack.skeleton)?;

    fs::create_dir_all(output.join("textures")).context("creating the output folder")?;
    let bytes = pack.to_bytes();
    fs::write(output.join("rider.bbsk"), &bytes)?;
    fs::write(output.join("graph.json"), serde_json::to_vec_pretty(&graph)?)?;
    fs::write(output.join("attach.json"), serde_json::to_vec_pretty(&attach)?)?;
    fs::write(output.join("parts.json"), serde_json::to_vec_pretty(&parts::parts_json(&names))?)?;
    parts::copy_textures(&input.join("textures"), &output.join("textures"))?;
    Ok(Summary { bones: pack.skeleton.len(), clips: pack.clips.len(), meshes: pack.meshes.len(), bytes: bytes.len() })
}

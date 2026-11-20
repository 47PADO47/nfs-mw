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
//! Output, in `files/RIDER/` of the mod (never committed): `rider.bbsk` (the pack), `graph.json` (the supermoto's blend tree),
//! `falls.json` (the fall and get-up animation, [`falls`]), `graph_<type>.json` (the tree of each other bike type whose clips the export has) with `styles.json` (which graph each
//! type has and the numbers of its reverse walk), `attach.json` (seat, pegs, grips, the limbs to solve and the numbers that
//! drive the animation), `parts.json` (meshes and their pictures) and the pictures.
//!
//! Each bike's own seat, grips and pegs, and the name of its clip set, are not in that folder: `ride6-bike-import` writes them
//! per car ([`contacts`], [`style`]).

pub mod acl_clips;
pub mod attach;
pub mod body_section;
pub mod contacts;
pub mod convert;
pub mod falls;
pub mod gear;
pub mod graph;
pub mod graph_fingers;
pub mod graph_reverse;
pub mod graph_sets;
pub mod graph_stand;
pub mod grip_span;
pub mod parts;
pub mod raw;
pub mod seat_surface;
pub mod style;

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
    /// The bike types that have a graph of their own.
    pub styles: Vec<&'static str>,
    /// Whether `falls.json` (the fall and get-up animation) was written.
    pub falls: bool,
}

/// Converts the export in `input` into `output`. Every graph carries the lean sets of all three riding styles
/// ([`graph::graph_selectable`]); the game picks one by the player's setting.
pub fn import(input: &Path, output: &Path) -> Result<Summary> {
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
    let graph = graph::graph_selectable(&pack, &graph_sets::MOTARD)?;
    graph.validate(&pack.clips, &pack.skeleton)?;
    let styles = write_styles(&pack, output)?;
    let falls = write_falls(&pack, output)?;

    fs::create_dir_all(output.join("textures")).context("creating the output folder")?;
    let bytes = pack.to_bytes();
    fs::write(output.join("rider.bbsk"), &bytes)?;
    fs::write(output.join("graph.json"), serde_json::to_vec_pretty(&graph)?)?;
    fs::write(output.join("attach.json"), serde_json::to_vec_pretty(&attach)?)?;
    fs::write(output.join("parts.json"), serde_json::to_vec_pretty(&parts::parts_json(&names))?)?;
    parts::copy_textures(&input.join("textures"), &output.join("textures"))?;
    Ok(Summary {
        bones: pack.skeleton.len(),
        clips: pack.clips.len(),
        meshes: pack.meshes.len(),
        bytes: bytes.len(),
        styles,
        falls,
    })
}

/// Writes `falls.json`, the animation of the rider's fall and get-up ([`falls::fall_set`]); whether it was written. A pack without the
/// fall clips (an export made without the `Ragdoll` folder) gets none, with a warning: the game then lets the rider fall limp.
fn write_falls(pack: &blackbox_skeleton::Pack, output: &Path) -> Result<bool> {
    let set = match falls::fall_set(pack).and_then(|set| {
        set.validate(&pack.clips, &pack.skeleton)?;
        Ok(set)
    }) {
        Ok(set) => set,
        Err(error) => {
            eprintln!("warning: no fall animation: {error:#}");
            return Ok(false);
        }
    };
    fs::create_dir_all(output).context("creating the output folder")?;
    fs::write(output.join("falls.json"), serde_json::to_vec_pretty(&set)?)?;
    Ok(true)
}

/// Writes `graph_<type>.json` for every bike type whose clips the pack has, and `styles.json` naming them; returns the types'
/// names. A type with a clip missing is left out with a warning (the pack comes from an export without that type's folder).
fn write_styles(pack: &blackbox_skeleton::Pack, output: &Path) -> Result<Vec<&'static str>> {
    fs::create_dir_all(output).context("creating the output folder")?;
    let mut entries = Vec::new();
    for set in graph_sets::SETS {
        let graph = match graph::graph_selectable(pack, set).and_then(|graph| {
            graph.validate(&pack.clips, &pack.skeleton)?;
            Ok(graph)
        }) {
            Ok(graph) => graph,
            Err(error) => {
                eprintln!("warning: no graph for the {} bike type: {error:#}", set.name);
                continue;
            }
        };
        fs::write(output.join(style::graph_file(set)), serde_json::to_vec_pretty(&graph)?)?;
        let walk = graph_reverse::loop_seconds(pack, &set.walk);
        entries.push((set, style::entry(set, walk)));
    }
    let table = style::table(&entries);
    fs::write(output.join(style::FILE), serde_json::to_vec_pretty(&table)?)?;
    Ok(entries.iter().map(|(set, _)| set.name).collect())
}

/// Adds the animations of the export in `input` (a folder with `skeleton.json` and `*.r6ra` and/or `acl/`, made by the export tool's
/// `skeleton`, `anims` and `acl` modes) to the rider already in `output`, and writes every graph again. The clips the pack has
/// already keep their data. Nothing else of the rider is read again: a rider imported before a clip set grew gets it without the
/// whole export. When the folder also has `bike_skeleton.json` (the reference bike's), `attach.json` is made again with it.
pub fn merge(input: &Path, output: &Path) -> Result<Summary> {
    let read = |name: &str| fs::read(input.join(name)).with_context(|| format!("reading {name}"));
    let rider_json = raw::parse_skeleton(&read("skeleton.json")?)?;
    let (skeleton, _) = convert::skeleton(&rider_json)?;
    let mut pack = blackbox_skeleton::Pack::parse(
        &fs::read(output.join("rider.bbsk")).context("reading rider.bbsk of the rider to add to")?,
    )
    .context("rider.bbsk")?;
    anyhow::ensure!(
        pack.skeleton.len() == skeleton.len()
            && (0..skeleton.len()).all(|b| pack.skeleton.bones()[b].name == skeleton.bones()[b].name),
        "the export's skeleton is not the rider's"
    );
    let (anims, undecoded) = acl_clips::collect(input, &rider_json)?;
    for message in &undecoded {
        eprintln!("warning: an ACL clip was left out: {message}");
    }
    for clip in convert::clips(&anims, &skeleton)? {
        if pack.clip_index(&clip.name).is_none() {
            pack.clips.push(clip);
        }
    }
    let attach_path = output.join("attach.json");
    // With the reference bike's skeleton (`bike_skeleton.json`) attach.json is made again, with where the hands hold the grips; without
    // it the file is kept and only the throttle's wrist is added.
    let bike_skeleton = input.join("bike_skeleton.json");
    if bike_skeleton.is_file() {
        let bike_json = raw::parse_skeleton(&fs::read(&bike_skeleton).context("reading bike_skeleton.json")?)?;
        let reference = pack
            .clips
            .iter()
            .find(|c| c.name == attach::REFERENCE_CLIP)
            .with_context(|| format!("the animation {} is not in the rider", attach::REFERENCE_CLIP))?
            .sample(&pack.skeleton, 0.0, false);
        let attach = attach::attach_json(&pack.skeleton, &reference, &bike_json)?;
        fs::write(&attach_path, serde_json::to_vec_pretty(&attach)?)?;
    } else {
        let mut attach: serde_json::Value =
            serde_json::from_slice(&fs::read(&attach_path).context("reading attach.json")?).context("attach.json")?;
        if let Some(map) = attach.as_object_mut() {
            map.insert("wrist_throttle".to_owned(), attach::wrist_throttle());
        }
        fs::write(&attach_path, serde_json::to_vec_pretty(&attach)?)?;
    }
    let graph = graph::graph_selectable(&pack, &graph_sets::MOTARD)?;
    graph.validate(&pack.clips, &pack.skeleton)?;
    let styles = write_styles(&pack, output)?;
    let bytes = pack.to_bytes();
    fs::write(output.join("rider.bbsk"), &bytes)?;
    fs::write(output.join("graph.json"), serde_json::to_vec_pretty(&graph)?)?;
    Ok(Summary {
        bones: pack.skeleton.len(),
        clips: pack.clips.len(),
        meshes: pack.meshes.len(),
        bytes: bytes.len(),
        styles,
        falls: output.join("falls.json").is_file(),
    })
}

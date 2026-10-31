//! The animations RIDE 6 stores with the Animation Compression Library (ACL), decoded here instead of by the export tool.
//!
//! The export tool (`R6ToolRider acl <list> <folder>`) cannot decode them without a native library. It writes, per
//! sequence, the compressed buffer `<name>.acl` and `<name>.meta.json` (frame count, length, the track to bone map, the
//! notifies; see `docs/formats/ride6-rider.md`). [`read_folder`] decodes them with [`blackbox_acl`] into the same
//! [`RawClip`] the sampled `R6RA` file holds, so the rest of the import does not care where a clip came from.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use blackbox_acl::{Qvv, Tracks};
use serde::Deserialize;

use super::raw::{JsonSkeleton, RawClip};

/// One notify of the meta file; `link` is Unreal's link method (`Absolute`, `Relative` for a fraction of the length).
#[derive(Deserialize)]
struct MetaNotify {
    name: String,
    link: String,
    time: f32,
}

/// `<name>.meta.json`: the fields of the animation sequence the compressed buffer does not hold.
#[derive(Deserialize)]
struct Meta {
    name: String,
    num_frames: usize,
    length: f32,
    rate_scale: f32,
    /// `AAT_None`, `AAT_LocalSpaceBase` or `AAT_RotationOffsetMeshSpace`.
    additive: String,
    base_name: String,
    base_frame: i32,
    root_motion: bool,
    /// Skeleton bone of each track.
    track_to_bone: Vec<usize>,
    notifies: Vec<MetaNotify>,
}

/// The skeleton's reference pose as ACL transforms, indexed by bone.
fn reference_pose(skeleton: &JsonSkeleton) -> Vec<Qvv> {
    skeleton.bones.iter().map(|b| Qvv { rotation: b.q, translation: b.t, scale: b.s }).collect()
}

/// Decodes one compressed buffer into a raw clip (Unreal's axes and centimetres, like every clip of the `R6RA` file).
///
/// A bone the clip has no track for keeps the skeleton's reference pose, as in the sampled file.
pub fn decode_clip(bytes: &[u8], meta_json: &[u8], skeleton: &JsonSkeleton) -> Result<RawClip> {
    let meta: Meta = serde_json::from_slice(meta_json).context("the meta file")?;
    ensure!(meta.additive == "AAT_None", "{}: additive ACL clips are not supported", meta.name);
    let tracks = Tracks::parse(bytes).with_context(|| format!("{}: the ACL buffer", meta.name))?;
    ensure!(
        tracks.num_tracks() == meta.track_to_bone.len(),
        "{}: {} tracks but a map of {}",
        meta.name,
        tracks.num_tracks(),
        meta.track_to_bone.len()
    );
    ensure!(
        tracks.num_frames() == meta.num_frames,
        "{}: {} frames, the sequence says {}",
        meta.name,
        tracks.num_frames(),
        meta.num_frames
    );
    let bones = skeleton.bones.len();
    let reference = reference_pose(skeleton);
    let mut track_of_bone = vec![None; bones];
    let mut defaults = Vec::with_capacity(meta.track_to_bone.len());
    for (track, &bone) in meta.track_to_bone.iter().enumerate() {
        ensure!(bone < bones, "{}: track {track} is on bone {bone}, the skeleton has {bones}", meta.name);
        track_of_bone[bone] = Some(track);
        defaults.push(reference[bone]);
    }
    let frames = tracks.decode_all(Some(&defaults)).with_context(|| format!("{}: decoding", meta.name))?;
    let mut keys = Vec::with_capacity(frames.len() * bones);
    for frame in &frames {
        for (bone, rest) in reference.iter().enumerate() {
            let q = track_of_bone[bone].map_or(*rest, |t| frame[t]);
            let (r, t) = (q.rotation, q.translation);
            keys.push([r[0], r[1], r[2], r[3], t[0], t[1], t[2]]);
        }
    }
    let scales = (0..bones).map(|b| track_of_bone[b].map_or(reference[b].scale, |t| frames[0][t].scale)).collect();
    let notifies = meta
        .notifies
        .iter()
        .map(|n| (n.name.clone(), if n.link.contains("Relative") { n.time * meta.length } else { n.time }))
        .collect();
    Ok(RawClip {
        name: meta.name,
        length: meta.length,
        frames: frames.len(),
        rate_scale: meta.rate_scale,
        additive: 0,
        base_name: meta.base_name,
        base_frame: meta.base_frame,
        root_motion: meta.root_motion,
        notifies,
        scales,
        keys,
    })
}

/// Every `<name>.acl` of `dir` with its `<name>.meta.json`, in name order. A clip that cannot be decoded is reported on
/// the second list and left out.
pub fn read_folder(dir: &Path, skeleton: &JsonSkeleton) -> Result<(Vec<RawClip>, Vec<String>)> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "acl"))
        .collect();
    paths.sort();
    let (mut clips, mut failed) = (Vec::new(), Vec::new());
    for path in paths {
        let meta_path = path.with_extension("meta.json");
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let meta = fs::read(&meta_path).with_context(|| format!("reading {}", meta_path.display()))?;
        match decode_clip(&bytes, &meta, skeleton) {
            Ok(clip) => clips.push(clip),
            Err(e) => failed.push(format!("{}: {e:#}", path.display())),
        }
    }
    if clips.is_empty() && !failed.is_empty() {
        bail!("no ACL clip could be decoded: {}", failed.join("; "));
    }
    Ok((clips, failed))
}

/// Every animation of an export folder: `riding.r6ra` (the sampled sequences), any other `*.r6ra` next to it (for
/// example the fall and get-up sequences), and the ACL buffers of `acl/`. A name found twice keeps its first clip.
/// The second value lists the ACL clips that could not be decoded.
pub fn collect(input: &Path, skeleton: &JsonSkeleton) -> Result<(super::raw::RawAnims, Vec<String>)> {
    let mut files: Vec<_> = fs::read_dir(input)
        .with_context(|| format!("reading {}", input.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "r6ra"))
        .collect();
    files.sort_by_key(|p| (p.file_name().is_none_or(|n| n != "riding.r6ra"), p.clone()));
    let mut all: Option<super::raw::RawAnims> = None;
    for path in &files {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let anims = super::raw::parse_anims(&bytes).with_context(|| path.display().to_string())?;
        let Some(merged) = all.as_mut() else {
            all = Some(anims);
            continue;
        };
        ensure!(merged.bone_count == anims.bone_count, "{} has another skeleton", path.display());
        merged.clips.extend(anims.clips);
    }
    let mut all = all.context("no R6RA file in the export folder")?;
    let mut failed = Vec::new();
    if input.join("acl").is_dir() {
        let (clips, bad) = read_folder(&input.join("acl"), skeleton)?;
        all.clips.extend(clips);
        failed = bad;
    }
    let mut seen = std::collections::HashSet::new();
    all.clips.retain(|c| seen.insert(c.name.clone()));
    Ok((all, failed))
}

#[cfg(test)]
#[path = "acl_clips_tests.rs"]
mod tests;

//! `dump-anim`: the skeletons and animation banks of one bundle as text, and the decoded samples of one animation.

use std::fmt::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use blackbox_anim::codec::{Channel, DELTA_QUAT, read_leaf};
use blackbox_anim::{Animation, Bank, Compound, DOFS_PER_BONE, Error as AnimError, Object, Skeleton};
use nfsmw_data::anim::AnimObjects;
use nfsmw_data::game::open_install;

/// How many bones of each skeleton are printed; the rest are counted.
const BONES_SHOWN: usize = 8;

/// The `dump-anim` command: prints the bundle's skeletons and banks, then the samples of `bank` when it is given.
pub fn run(game_dir: Option<&Path>, path: &str, bank: Option<&str>, sample: Option<u32>) -> Result<()> {
    let dir = open_install(game_dir)?;
    let objects = AnimObjects::load(&dir, path)?;
    let target = bank.map(|name| find_animation(&objects, name)).transpose()?;
    let mut out = String::new();
    describe_skeletons(&mut out, &objects.skeletons);
    describe_banks(&mut out, &objects.banks);
    if let (Some((object, animation)), Some(name)) = (target, bank) {
        describe_animation(&mut out, &objects.skeletons, object, animation, name, sample)?;
    }
    print!("{out}");
    Ok(())
}

/// The first animation called `name` in the bundle's banks, with the object its records live in.
fn find_animation<'a>(objects: &'a AnimObjects, name: &str) -> Result<(&'a Object, &'a Animation)> {
    let found = objects.banks.iter().find_map(|(object, bank)| Some((object, bank.find(name)?)));
    match found {
        Some(found) => Ok(found),
        None => bail!("no animation {name:?} in this bundle (the banks list the names)"),
    }
}

/// One line per skeleton, then its first bones with their parents.
fn describe_skeletons(out: &mut String, skeletons: &[Skeleton]) {
    let _ = writeln!(out, "skeletons: {}", skeletons.len());
    for skeleton in skeletons {
        let bones = skeleton.bones.len();
        let _ = writeln!(out, "  {} (checksum {:#06x}): {bones} bones", skeleton.name, skeleton.checksum);
        for (index, bone) in skeleton.bones.iter().take(BONES_SHOWN).enumerate() {
            let parent = bone.parent.map_or("-".to_string(), |parent| parent.to_string());
            let _ = writeln!(out, "    bone {index:>3} {:<24} parent {parent}", bone.name);
        }
        let hidden = bones.saturating_sub(BONES_SHOWN);
        if hidden > 0 {
            let _ = writeln!(out, "    ... {hidden} more");
        }
    }
}

/// The banks, each animation with its frame count, frame rate and child codec types.
fn describe_banks(out: &mut String, banks: &[(Object, Bank)]) {
    let animations: usize = banks.iter().map(|(_, bank)| bank.animations.len()).sum();
    let _ = writeln!(out, "banks: {} ({animations} animations)", banks.len());
    for (index, (object, bank)) in banks.iter().enumerate() {
        let _ = writeln!(out, "  bank {index}: {} animations", bank.animations.len());
        for animation in bank.iter() {
            let name = &animation.name;
            match Compound::read(object, animation.offset) {
                Ok(compound) => {
                    let types: Vec<String> = compound.children.iter().map(|c| c.type_id.to_string()).collect();
                    let _ = writeln!(
                        out,
                        "    {name}: {} frames at {} fps, child types [{}]",
                        compound.frames,
                        compound.fps,
                        types.join(" ")
                    );
                }
                Err(e) => {
                    let _ = writeln!(out, "    {name}: {e}");
                }
            }
        }
    }
}

/// The animation's summary and, when `samples` is given, that many evenly spaced frames of its decoded dofs. The
/// dof buffer starts from the bind pose of the skeleton the animation drives (matched by checksum), when the bundle
/// has one.
fn describe_animation(
    out: &mut String,
    skeletons: &[Skeleton],
    object: &Object,
    animation: &Animation,
    name: &str,
    samples: Option<u32>,
) -> Result<()> {
    let compound = Compound::read(object, animation.offset).with_context(|| format!("compound of {name}"))?;
    let _ = writeln!(
        out,
        "animation {name}: {} frames at {} fps, {} children",
        compound.frames,
        compound.fps,
        compound.children.len()
    );
    let Some(count) = samples else { return Ok(()) };

    let mut channels: Vec<(u16, Option<Box<dyn Channel + Send + Sync>>)> = Vec::new();
    for child in &compound.children {
        match read_leaf(object, child.type_id, child.offset) {
            Ok(channel) => channels.push((child.type_id, Some(channel))),
            Err(AnimError::Unsupported(_)) => channels.push((child.type_id, None)),
            Err(e) => {
                return Err(anyhow::Error::new(e).context(format!("the channel at {:#x} of {name}", child.offset)));
            }
        }
    }

    let skeleton = skeletons.iter().find(|skeleton| skeleton.checksum == compound.checksum);
    let bind = skeleton.map(Skeleton::bind_pose);
    let widest = channels
        .iter()
        .filter_map(|(_, channel)| channel.as_ref())
        .flat_map(|channel| channel.dofs().iter().map(|&dof| usize::from(dof) + 4))
        .max()
        .unwrap_or(0);
    let len = match &bind {
        Some(pose) => pose.bones.len() * DOFS_PER_BONE,
        None => widest.next_multiple_of(DOFS_PER_BONE),
    };
    let last_frame = f32::from(compound.frames.saturating_sub(1));
    let spacing = count.max(2) - 1;
    for sample in 0..count {
        let frame = last_frame * sample as f32 / spacing as f32;
        let mut dofs = vec![0.0f32; len];
        if let Some(pose) = &bind {
            pose.write_dofs(&mut dofs);
        }
        for channel in channels.iter().filter_map(|(_, channel)| channel.as_deref()) {
            channel.sample(frame, &mut dofs);
        }
        let _ = writeln!(out, "  frame {frame:.2}:");
        for (type_id, channel) in &channels {
            let Some(channel) = channel else {
                let _ = writeln!(out, "    unsupported type {type_id}");
                continue;
            };
            let width = if *type_id == DELTA_QUAT { 4 } else { 1 };
            for &dof in channel.dofs() {
                let at = usize::from(dof);
                let values: Vec<String> =
                    dofs.get(at..at + width).unwrap_or_default().iter().map(|v| format!("{v:.4}")).collect();
                let bone = at / DOFS_PER_BONE;
                let slot = at % DOFS_PER_BONE;
                let label = skeleton.and_then(|s| s.bones.get(bone)).map_or(String::new(), |b| format!(" {}", b.name));
                let _ = writeln!(out, "    dof {at:>4} (bone {bone}{label}, slot {slot}): [{}]", values.join(", "));
            }
        }
    }
    Ok(())
}

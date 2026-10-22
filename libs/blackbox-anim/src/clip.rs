//! An animation clip: one named animation of a bank, read as a compound and its child channels, and sampled by time
//! into a DOF buffer or a pose.
//!
//! The children all write into the same buffer, and the last child is visited first, so an earlier child wins where
//! two children drive the same slot (`docs/specs/eagl-animation-codecs.md` section 2). The time base is frames at the
//! compound's frame rate, and the length is the frame count over that rate (`docs/specs/eagl-animation-bank.md`
//! sections 10.2 and 10.3).

use crate::codec::{self, Channel};
use crate::pose::DOFS_PER_BONE;
use crate::{Animation, Bank, Compound, Error, Object, Pose, Result, TypeId};

/// One named animation, ready to sample.
pub struct Clip {
    /// The animation name, for example `ArrestF02_Cuffs_s`.
    pub name: String,
    /// The target checksum of the animation: the skeleton it drives.
    pub checksum: u16,
    /// Frames per second.
    pub fps: f32,
    /// The length in frames.
    pub frames: u16,
    /// The type ids of the child channels whose codec is not implemented. Such a child writes nothing, so the slots
    /// it drives keep the values they had before sampling.
    pub skipped: Vec<u16>,
    channels: Vec<Box<dyn Channel + Send + Sync>>,
    max_dof: usize,
}

impl Clip {
    /// Reads the clip for `animation`, a bank entry of `object`. The entry's record must be a compound.
    ///
    /// A child whose codec is not implemented is left out and its type id goes into [`Clip::skipped`]. Any other
    /// child error (a corrupt record) fails the whole clip.
    pub fn read(object: &Object, animation: &Animation) -> Result<Self> {
        let compound = Compound::read(object, animation.offset)?;
        let mut channels = Vec::with_capacity(compound.children.len());
        let mut skipped = Vec::new();
        let mut max_dof = 0;
        for child in &compound.children {
            let channel = match codec::read_leaf(object, child.type_id, child.offset) {
                Ok(channel) => channel,
                Err(Error::Unsupported(_)) => {
                    skipped.push(child.type_id);
                    continue;
                }
                Err(error) => return Err(error),
            };
            let width = floats_per_dof(child.type_id);
            let reach = channel.dofs().iter().map(|&dof| usize::from(dof) + width).max().unwrap_or(0);
            max_dof = max_dof.max(reach);
            channels.push(channel);
        }
        Ok(Self {
            name: animation.name.clone(),
            checksum: animation.checksum,
            fps: compound.fps,
            frames: compound.frames,
            skipped,
            channels,
            max_dof,
        })
    }

    /// The length in seconds: the frame count over the frame rate.
    pub fn duration_secs(&self) -> f32 {
        f32::from(self.frames) / self.fps
    }

    /// The frame at `secs`: the time times the frame rate, clamped to `[0, frames - 1]`. A NaN time is frame 0.
    pub fn frame_at(&self, secs: f32) -> f32 {
        let frame = secs * self.fps;
        if frame.is_nan() {
            return 0.0;
        }
        let last = f32::from(self.frames.saturating_sub(1));
        frame.clamp(0.0, last)
    }

    /// Writes every channel's values at `secs` into `dofs`, last child first. A slot that no channel drives is left
    /// as it was. Dof indices past the end of `dofs` are skipped.
    pub fn sample(&self, secs: f32, dofs: &mut [f32]) {
        let frame = self.frame_at(secs);
        for channel in self.channels.iter().rev() {
            channel.sample(frame, dofs);
        }
    }

    /// The pose at `secs`, starting from `base`: the slots the clip does not drive keep their `base` values.
    pub fn sample_pose(&self, secs: f32, base: &Pose) -> Pose {
        let mut dofs = vec![0.0; base.bones.len() * DOFS_PER_BONE];
        base.write_dofs(&mut dofs);
        self.sample(secs, &mut dofs);
        Pose::from_dofs(&dofs)
    }

    /// One past the largest float slot the clip writes in a DOF buffer: the largest dof index plus the floats that
    /// index writes. Zero when no channel was read. The bone count the clip needs is this value over
    /// [`DOFS_PER_BONE`], rounded up.
    pub fn max_dof(&self) -> usize {
        self.max_dof
    }

    /// The decoded channels, in child order.
    pub fn channels(&self) -> &[Box<dyn Channel + Send + Sync>] {
        &self.channels
    }
}

/// Every animation of the bank of `object`, in bank order. A bank with no animations gives an empty list.
pub fn clips(object: &Object) -> Result<Vec<Clip>> {
    let bank = Bank::read(object)?;
    bank.iter().map(|animation| Clip::read(object, animation)).collect()
}

/// The number of consecutive floats that one dof index of a channel of `type_id` writes: four for a quaternion, three
/// for a DeltaF3 vector and one for a scalar (`docs/specs/eagl-animation-codecs.md` sections 6 to 9).
fn floats_per_dof(type_id: u16) -> usize {
    match TypeId::from_u16(type_id) {
        Some(TypeId::DeltaQuat | TypeId::DeltaQ) => 4,
        Some(TypeId::DeltaF3) => 3,
        _ => 1,
    }
}

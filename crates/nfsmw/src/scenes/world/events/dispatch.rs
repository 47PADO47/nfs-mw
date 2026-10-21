//! Shared ordered dispatch of installed action lists and local stimuli.

use super::Effect;
use anyhow::{Context, Result, bail};
use blackbox_attrib::vlt_hash;
use blackbox_sequence::{Player, QueueMode};
use glam::Vec3;
use nfsmw_data::world::sequences::{Sequence, WorldEvent};

pub(in super::super) fn drain(
    sequence: &Sequence,
    player: &mut Player,
    velocity: Option<Vec3>,
    out: &mut Vec<Effect>,
) -> Result<()> {
    let package = &sequence.package;
    let mut remaining = 4096;
    while let Some(emission) = player.pop() {
        let mut pending: Vec<_> =
            package.event_lists.get(&emission.list).context("missing emitted list")?.iter().rev().collect();
        while let Some(event) = pending.pop() {
            if remaining == 0 {
                bail!("event dispatch cycle exceeded 4096 events");
            }
            remaining -= 1;
            let event = WorldEvent::decode(event)?.resolve_strings(package);
            if let WorldEvent::FireList { list, delayed } = event {
                if delayed {
                    bail!("delayed EFireEventList needs deferred context");
                }
                let children = package.event_lists.get(&list).context("EFireEventList has missing list")?;
                pending.extend(children.iter().rev());
                continue;
            }
            if let WorldEvent::Stimulus { system, stimulus, queue_mode } = event {
                let target = match system {
                    0 => None,
                    h if h == vlt_hash("this") => Some(emission.system),
                    h => Some(h),
                };
                player.stimulate(package, target, stimulus, queue(queue_mode)?, None)?;
                continue;
            }
            out.push(Effect { event, velocity: emission.context.and(velocity) });
        }
    }
    Ok(())
}

pub(in super::super) fn queue(hash: u32) -> Result<QueueMode> {
    match hash {
        0 | 0x102A_BF91 => Ok(QueueMode::Allow),
        0xCC7F_660A => Ok(QueueMode::Disable),
        0x669E_C6BC => Ok(QueueMode::Shallow),
        0x3C56_741D => Ok(QueueMode::Flush),
        0x936F_462E => Ok(QueueMode::Abort),
        _ => bail!("unknown queue mode {hash:08X}"),
    }
}

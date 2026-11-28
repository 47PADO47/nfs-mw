//! What the NIS frame capture (`parity/nis_capture.rs`, `nis/capture.rs`) reads of the playing scene: its clock, its
//! camera sample, and each car channel with the car it drives.

use blackbox_chunk::ice::CameraSample;

use super::{CarSpot, NisPlayer, SCENE_FPS, Target};
use crate::scenes::world::scene_space::Anchor;

/// One car channel of the playing scene now.
pub struct ChannelNow {
    pub slot: usize,
    /// The real car the channel drives, once linked.
    pub target: Option<Target>,
    /// How the world placed this car after applying scene events.
    pub motion: Option<&'static str>,
    /// The pose the scene data gives the channel, in render space and before the ground lift.
    pub spot: CarSpot,
    /// The scene time at which the channel's clips end.
    pub clip_end: f32,
}

/// The playing scene now, as the capture records it.
pub struct SceneNow {
    pub name: String,
    pub scene_time: f32,
    pub elapsed: f32,
    pub duration: f32,
    /// The scene frame (30 a second) of `elapsed`.
    pub frame: u32,
    pub anchor: Anchor,
    pub camera: Option<CameraSample>,
    pub channels: Vec<ChannelNow>,
}

impl NisPlayer {
    /// The playing scene now; `None` when none plays.
    pub fn capture_now(&self) -> Option<SceneNow> {
        let playing = self.play.as_ref()?;
        let time = playing.run.scene_time();
        let links = playing.links.as_deref().unwrap_or_default();
        let channels = playing
            .cars
            .iter()
            .map(|car| {
                let target = links.iter().find(|(slot, _)| *slot == car.slot).map(|link| link.1);
                ChannelNow {
                    slot: car.slot,
                    target,
                    motion: target.and_then(|target| {
                        playing.memory.crash.mode(target).or_else(|| playing.memory.coasts.mode(target))
                    }),
                    spot: car.spot(&playing.anchor, time),
                    clip_end: car.end_time(),
                }
            })
            .collect();
        Some(SceneNow {
            name: playing.name.clone(),
            scene_time: time,
            elapsed: playing.run.elapsed(),
            duration: playing.run.clock().duration(),
            frame: (playing.run.elapsed() * SCENE_FPS).round() as u32,
            anchor: playing.anchor,
            camera: self.sample(),
            channels,
        })
    }
}

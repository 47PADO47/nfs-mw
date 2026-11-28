//! The world's side of the NIS frame capture (`parity/nis_capture.rs`): the fixed scene step, and one record line per
//! scene frame with what the scene data asks for next to what the frame draws. The frames after a scene's end are
//! captured too (`<scene>-end-<n>.png`), to show the hand-over to the race.

use std::cell::{Cell, RefCell};

use glam::{Quat, Vec3};
use serde_json::{Value, json};

use super::cars::Target;
use super::player::SCENE_FPS;
use super::player::capture_api::SceneNow;
use crate::parity::{career_trace, nis_capture};
use crate::scenes::world::WorldScene;
use crate::scenes::world::scene_space::place_on_car;
use crate::viewer::camera::Lens;

/// Other cars within this distance of the camera are listed in the record, metres.
const NEARBY: f32 = 250.0;
/// Frames captured after a scene ends, and the drawn frames between them.
const AFTER_END_SHOTS: u32 = 4;
const AFTER_END_STEP: u32 = 20;

#[derive(Default)]
struct Last {
    scene: String,
    frame: Option<u32>,
    positions: Vec<(usize, Vec3)>,
    /// The scene that ended and the frames drawn since.
    after: Option<(String, u32)>,
}

thread_local! {
    static LAST: RefCell<Last> = RefCell::new(Last::default());
    static CAREER_SAMPLE: Cell<Option<(OpeningScene, u32)>> = const { Cell::new(None) };
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OpeningScene {
    DDay,
    RollingStart,
}

fn career_sample_key(name: &str, time: f32) -> Option<(OpeningScene, u32)> {
    let scene = if name.eq_ignore_ascii_case("IntroNisDD") {
        OpeningScene::DDay
    } else if name.eq_ignore_ascii_case("IntroNis10") {
        OpeningScene::RollingStart
    } else {
        return None;
    };
    Some((scene, (time * 2.0).floor() as u32))
}

impl WorldScene {
    /// The time a captured scene advances by this frame: one scene frame, or none while a frame is read back. Other
    /// scenes, and every scene when the capture is off, take the frame's real time `dt`.
    pub(in crate::scenes::world) fn nis_capture_step(&self, dt: f32) -> f32 {
        if !self.nis.playing_name().is_some_and(nis_capture::wants) {
            return dt;
        }
        match nis_capture::busy() {
            true => 0.0,
            false => 1.0 / SCENE_FPS,
        }
    }

    /// The scene `name` ended: the frames after it are captured, and the record says where the cars stand.
    pub(in crate::scenes::world) fn note_nis_capture_end(&self, name: &str) {
        if !nis_capture::wants(name) {
            return;
        }
        LAST.with_borrow_mut(|last| last.after = Some((name.to_owned(), 0)));
        let player =
            self.players.lead().map(|drive| pose_json(drive.render_pose().position, drive.render_pose().rotation));
        nis_capture::record(
            &json!({"kind": "end", "scene": name, "player": player, "others": self.nearby_cars(None, &[])}),
        );
        nis_capture::scene_ended(name);
    }

    /// Records the frame being built with the lens `lens`, and asks for its picture every so many scene frames.
    pub(in crate::scenes::world) fn record_nis_frame(&self, lens: &Lens, aspect: f32) {
        if career_trace::enabled() {
            self.record_opening_sample(lens);
        }
        if !nis_capture::enabled() {
            return;
        }
        let Some(now) = self.nis.capture_now().filter(|now| nis_capture::wants(&now.name)) else {
            return self.capture_after_end();
        };
        let fresh = LAST.with_borrow_mut(|last| {
            if last.scene != now.name {
                *last = Last { scene: now.name.clone(), ..Last::default() };
            }
            let fresh = last.frame != Some(now.frame);
            last.frame = Some(now.frame);
            last.after = None;
            fresh
        });
        if !fresh {
            return;
        }
        let shot = now.frame % nis_capture::every() == 0;
        if shot {
            nis_capture::request(&now.name, now.frame);
        }
        let line = self.frame_line(&now, lens, aspect, shot);
        nis_capture::record(&line);
    }

    fn record_opening_sample(&self, lens: &Lens) {
        let key =
            self.nis.playing_name().zip(self.nis.scene_time()).and_then(|(name, time)| career_sample_key(name, time));
        let previous = CAREER_SAMPLE.with(|last| last.replace(key));
        if key.is_none() || key == previous {
            return;
        }
        let Some(now) = self.nis.capture_now() else { return };
        let channels: Vec<_> = now
            .channels
            .iter()
            .map(|channel| {
                let drawn = channel.target.and_then(|target| self.drawn_pose(target));
                json!({
                    "channel": channel_name(channel.slot),
                    "target": channel.target.map(|target| format!("{target:?}")),
                    "motion": channel.motion,
                    "scene": pose_json(channel.spot.position, channel.spot.rotation),
                    "drawn": drawn.map(|(position, rotation)| pose_json(position, rotation)),
                })
            })
            .collect();
        career_trace::record(
            "nis_sample",
            json!({
                "scene": now.name,
                "scene_time": round(now.scene_time),
                "elapsed": round(now.elapsed),
                "camera": lens_json(lens),
                "channels": channels,
            }),
        );
    }

    /// The frames after a captured scene's end: [`AFTER_END_SHOTS`] of them, [`AFTER_END_STEP`] drawn frames apart.
    fn capture_after_end(&self) {
        let due = LAST.with_borrow_mut(|last| {
            let (scene, drawn) = last.after.as_mut()?;
            *drawn += 1;
            let shot = *drawn / AFTER_END_STEP;
            let name = (*drawn % AFTER_END_STEP == 0).then(|| format!("{scene}-end"))?;
            if shot >= AFTER_END_SHOTS {
                last.after = None;
            }
            Some((name, shot))
        });
        let Some((name, shot)) = due.filter(|_| !nis_capture::busy()) else { return };
        nis_capture::request(&name, shot);
    }

    fn frame_line(&self, now: &SceneNow, lens: &Lens, aspect: f32, shot: bool) -> Value {
        let previous = LAST.with_borrow(|last| last.positions.clone());
        let mut positions = Vec::new();
        let mut taken = Vec::new();
        let channels: Vec<Value> = now
            .channels
            .iter()
            .map(|channel| {
                let drawn = channel.target.and_then(|target| self.drawn_pose(target));
                let at = drawn.map_or(channel.spot.position, |pose| pose.0);
                let speed = previous
                    .iter()
                    .find(|(slot, _)| *slot == channel.slot)
                    .map(|(_, before)| (at - *before).length() * SCENE_FPS);
                positions.push((channel.slot, at));
                if let Some(Target::Traffic(index) | Target::Spawned(index)) = channel.target {
                    taken.push(index);
                }
                json!({
                    "channel": channel_name(channel.slot),
                    "target": channel.target.map(|target| format!("{target:?}")),
                    "scene": pose_json(channel.spot.position, channel.spot.rotation),
                    "drawn": drawn.map(|(position, rotation)| pose_json(position, rotation)),
                    "lift": drawn.map(|(position, _)| round(position.z - channel.spot.position.z)),
                    "speed": speed.map(round),
                    "treads": channel.target.and_then(|t| self.tread_sample(t)),
                    "motion": channel.motion,
                    "clip_end": round(channel.clip_end),
                })
            })
            .collect();
        LAST.with_borrow_mut(|last| last.positions = positions);
        let car = self.nis.camera_car().map(|spot| (spot.position, spot.rotation));
        let track = now.camera.as_ref().map(|sample| {
            json!({
                "eye": place_on_car(&now.anchor, sample.eye_space, sample.eye, car).map(v3),
                "look": place_on_car(&now.anchor, sample.look_space, sample.look, car).map(v3),
                "overlay": sample.overlay,
                "sim_speed": round(sample.sim_speed),
                "space": format!("{:?}/{:?}", sample.eye_space, sample.look_space),
                "fov_deg": round(f32::from(sample.fov) / 65536.0 * 360.0),
                "roll": sample.roll,
                "letterbox": round(sample.letterbox),
            })
        });
        let mut staged = self.nis.car_spots();
        let linked = staged.len();
        self.stage_extra_intro_cars(&mut staged);
        let extras: Vec<_> = staged[linked..]
            .iter()
            .map(|(t, _)| json!({"target":format!("{t:?}"),"treads":self.tread_sample(*t)}))
            .collect();
        let scene_lens = self.nis.lens(aspect).map(|lens| lens_json(&lens));
        json!({
            "kind": "frame",
            "scene": now.name,
            "frame": now.frame,
            "time": round(now.scene_time),
            "elapsed": round(now.elapsed),
            "duration": round(now.duration),
            "png": shot,
            "anchor": {"origin": v3(now.anchor.origin), "turn_deg": round(f32::from(now.anchor.turn) / 65536.0 * 360.0)},
            "camera": {"track": track, "scene_lens": scene_lens, "used": lens_json(lens)},
            "channels": channels,
            "extra_grid": extras,
            "others": self.nearby_cars(Some(lens.position), &taken),
        })
    }

    /// Vertical gaps at the displayed tread positions, queried only for a requested frame trace.
    fn tread_sample(&self, target: Target) -> Option<Value> {
        let (pose, model) = match target {
            Target::Player => {
                let car = self.players.lead()?;
                (car.render_pose(), car.car_name.as_str())
            }
            Target::Traffic(i) | Target::Spawned(i) => {
                let traffic = self.traffic.as_ref()?;
                (traffic.car_render_pose(i)?, traffic.scene_model_name(i)?)
            }
        };
        let (points, scale, visual) = self.target_treads(target)?;
        let gaps = std::array::from_fn::<_, 4, _>(|i| {
            let p = pose.position + pose.rotation * (points[i] + Vec3::Z * (pose.wheels[i].travel * scale));
            super::super::space::ground_below(self.residency.collision(), p.x, p.y, p.z + 1.5, p.z - 8.0)
                .map(|h| round(p.z - h))
        });
        Some(json!({"model":model,"mesh_treads":visual,"gap_m":gaps}))
    }

    /// Where the car of `target` is drawn now.
    fn drawn_pose(&self, target: Target) -> Option<(Vec3, Quat)> {
        let pose = match target {
            Target::Player => self.players.lead().map(|drive| drive.render_pose()),
            Target::Traffic(index) | Target::Spawned(index) => self.traffic.as_ref()?.car_render_pose(index),
        }?;
        Some((pose.position, pose.rotation))
    }

    /// The traffic cars that no channel takes, near `near` (all of them with `None`).
    fn nearby_cars(&self, near: Option<Vec3>, taken: &[usize]) -> Vec<Value> {
        let Some(traffic) = self.traffic.as_ref() else { return Vec::new() };
        let close = |at: Vec3| near.is_none_or(|eye| eye.distance(at) <= NEARBY);
        let cars = traffic.car_census().into_iter().filter(|(index, at, ..)| !taken.contains(index) && close(*at));
        cars.map(|(index, at, police, racer)| json!({"index": index, "at": v3(at), "police": police, "racer": racer}))
            .collect()
    }
}

/// `car1`..`car8` for slots 0 to 7, `cop1`..`cop8` for 8 to 15.
fn channel_name(slot: usize) -> String {
    match slot < 8 {
        true => format!("car{}", slot + 1),
        false => format!("cop{}", slot - 7),
    }
}

fn round(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}

fn v3(v: Vec3) -> [f32; 3] {
    [round(v.x), round(v.y), round(v.z)]
}

/// A position with the heading, pitch and roll of a car's rotation, degrees (the car's nose is its +x, its left +y).
fn pose_json(position: Vec3, rotation: Quat) -> Value {
    let forward = rotation * Vec3::X;
    let left = rotation * Vec3::Y;
    let up = rotation * Vec3::Z;
    json!({
        "at": v3(position),
        "heading": round(forward.y.atan2(forward.x).to_degrees()),
        "pitch": round(forward.z.clamp(-1.0, 1.0).asin().to_degrees()),
        "roll": round(left.z.atan2(up.z).to_degrees()),
        "upright": up.z > 0.0,
    })
}

fn lens_json(lens: &Lens) -> Value {
    json!({"position": v3(lens.position), "forward": v3(lens.forward), "fov_y_deg": round(lens.fov_y_degrees)})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn career_opening_samples_use_half_second_scene_time_buckets() {
        let origin = career_sample_key("IntroNisDD", 0.0);
        assert!(origin.is_some());
        assert!(origin == career_sample_key("intronisdd", 0.499));
        assert!(origin != career_sample_key("IntroNisDD", 0.5));
        assert!(origin != career_sample_key("IntroNis10", 0.0));
        assert!(career_sample_key("IntroNis11", 0.0).is_none());
    }
}

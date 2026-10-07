//! The streamed city with a free-fly camera.

mod ground;
mod residency;
mod resident;
mod visibility;

use anyhow::{Context, Result};
use blackbox_render::{FrameParams, Instance, Renderer};
use game_install::GameDir;
use glam::Vec3;
use nfsmw_data::world::{DEFAULT_TRACK, Streamer, WorldIndex};

use crate::viewer::{Input, Scene, camera::FlyCamera};
use residency::Residency;

pub struct Options {
    pub at: Option<[f32; 2]>,
    pub height: f32,
    pub heading: f32,
    pub pitch: f32,
    pub load_radius: f32,
    pub wait_for_load: bool,
}

pub struct WorldScene {
    track: String,
    camera: FlyCamera,
    residency: Residency,
    /// Start position, for `ready()`.
    start: [f32; 2],
    /// Height above the ground to place the camera at once the start area has loaded.
    start_height: f32,
    grounded: bool,
    wait_for_load: bool,
    visible: Vec<Instance>,
}

const CLEAR: [f32; 3] = [0.55, 0.63, 0.72];

impl WorldScene {
    pub fn open(dir: &GameDir, options: Options) -> Result<Self> {
        let index = WorldIndex::open(dir, DEFAULT_TRACK)?;
        let stream = dir.resolve(&index.stream_file).with_context(|| format!("{} is missing", index.stream_file))?;
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get().clamp(2, 6));
        let streamer = Streamer::spawn(stream.to_path_buf(), index.sections.clone(), threads);
        let start = options.at.unwrap_or_else(|| index.centre());
        let camera = FlyCamera {
            position: Vec3::new(start[0], start[1], options.height),
            yaw: options.heading.to_radians(),
            pitch: options.pitch.to_radians(),
            speed: 40.0,
            far: options.load_radius * 2.0,
        };
        log::info!("camera starts at ({:.0}, {:.0}), {:.0} m above the ground", start[0], start[1], options.height);
        Ok(Self {
            track: index.track.clone(),
            camera,
            residency: Residency::new(index.sections, streamer, options.load_radius),
            start,
            start_height: options.height,
            grounded: false,
            wait_for_load: options.wait_for_load,
            visible: Vec::new(),
        })
    }

    fn draw_distance(&self) -> f32 {
        self.residency.load_radius * 1.4
    }
}

impl Scene for WorldScene {
    fn title(&self) -> String {
        format!("nfsmw — {}", self.track)
    }

    fn init(&mut self, _renderer: &mut Renderer) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, renderer: &mut Renderer, input: &Input, dt: f32) {
        self.camera.update(input, dt);
        let p = self.camera.position;
        self.residency.update(renderer, p.x, p.y);
        if !self.grounded && self.residency.complete_at(self.start[0], self.start[1]) {
            self.grounded = true;
            let ground = ground::height_near(self.residency.placed(), self.start[0], self.start[1], 150.0);
            self.camera.position.z = ground.unwrap_or(0.0) + self.start_height;
            log::info!("ground near the start: {ground:?}; camera at z = {:.0}", self.camera.position.z);
        }
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let view_proj = self.camera.view_proj(aspect);
        let distance = self.draw_distance();
        visibility::collect(self.residency.placed(), &view_proj, self.camera.position, distance, &mut self.visible);
        let params = FrameParams {
            view_proj,
            camera_position: self.camera.position,
            light_dir: Vec3::new(-0.35, -0.45, -1.0),
            clear_color: CLEAR,
            fog_start: distance * 0.45,
            fog_end: distance,
        };
        (params, &self.visible)
    }

    fn ready(&self) -> bool {
        let [x, y] = self.start;
        if self.wait_for_load { self.grounded && self.residency.complete_at(x, y) } else { self.residency.counts().2 }
    }

    fn status(&self) -> Option<String> {
        let (resident, loading, shared) = self.residency.counts();
        let p = self.camera.position;
        Some(format!(
            "{}tiles {resident} (+{loading} loading), {} drawn, at ({:.0}, {:.0}, {:.0})",
            if shared { "" } else { "loading shared sets… " },
            self.visible.len(),
            p.x,
            p.y,
            p.z
        ))
    }
}

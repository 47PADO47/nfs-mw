//! Movies (`MOVIES/*.vp6`): a scene that plays one full screen, and the plugin that shows a scene's picture.

mod framing;
mod playback;
mod player;
mod subtitles;

use anyhow::Result;
use bevy_app::{AppExit, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_window::{PrimaryWindow, Window};
use blackbox_gfx::{FrameParams, Instance, Projection, RenderBackend, UiLayer, UiMesh, UiTextureId, UiVertex};
use game_install::GameDir;
use glam::{Mat4, Vec3};

use crate::app::{FrameSet, Host};
use crate::gui::{OwnedPatch, UiOutput};
use crate::input::ActionState;
use crate::viewer::{Fullscreen, Scene};
use framing::fit_rect;
pub(crate) use playback::MoviePlayback;
pub(crate) use subtitles::show as show_subtitles;

/// Texture id of the movie picture; egui's are small numbers and the HUD's have a high bit.
const PICTURE: UiTextureId = UiTextureId(0x4D4F_5649_4500);

/// The movies in the install.
pub fn list(dir: &GameDir) -> Vec<String> {
    let mut movies = dir.files_in("MOVIES");
    movies.retain(|f| f.to_ascii_lowercase().ends_with(".vp6"));
    movies.sort();
    movies
}

/// A movie as a full-screen scene: the scene is over when the movie is.
pub struct MovieScene {
    playback: MoviePlayback,
}

impl MovieScene {
    pub fn open(dir: &GameDir, name: &str, start: f64) -> Result<Self> {
        Ok(Self { playback: MoviePlayback::open(dir, name, start)? })
    }
}

impl Scene for MovieScene {
    fn title(&self) -> String {
        format!("nfsmw — {}", self.playback.name())
    }

    fn init(&mut self, _renderer: &mut dyn RenderBackend) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, _renderer: &mut dyn RenderBackend, _input: &ActionState, dt: f32) {
        self.playback.advance(dt);
    }

    fn hud_state(&self) -> Option<crate::hud::HudState> {
        Some(crate::hud::HudState { visible: false, ..Default::default() })
    }

    fn ui(&mut self, ctx: &egui::Context) -> f32 {
        subtitles::show(ctx, self.playback.subtitle_lines());
        0.0
    }

    fn frame(&mut self, _aspect: f32) -> (FrameParams, &[Instance]) {
        let params = FrameParams {
            view: Mat4::IDENTITY,
            projection: Projection::Identity,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.0, 0.0, 0.0],
            fog: None,
            scattering: None,
            surface_light: None,
            // Nothing 3D is drawn, so there is no camera to follow from frame to frame.
            camera_cut: true,
            shadow_tuning: None,
        };
        (params, &[])
    }

    fn fullscreen(&mut self) -> Option<Fullscreen> {
        self.playback.picture()
    }

    fn take_clip(&mut self) -> Option<ea_audio::Pcm> {
        self.playback.take_clip()
    }

    fn finished(&self) -> bool {
        self.playback.finished()
    }
}

/// Shows the scene's [`Fullscreen`] picture over the window, and quits when the scene is over.
pub struct FullscreenPlugin;

impl Plugin for FullscreenPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(Update, present.in_set(FrameSet::Hud));
    }
}

fn present(
    mut host: NonSendMut<Host>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut out: ResMut<UiOutput>,
    mut exit: MessageWriter<AppExit>,
) {
    let _trace = blackbox_trace::scope("Movie: present fullscreen frame");
    if host.scene.finished() {
        // With the front end the flow moves on to the next scene; without it the program is over.
        if !host.flow_driven {
            exit.write(AppExit::Success);
        }
        return;
    }
    let Some(picture) = host.scene.fullscreen() else { return };
    if let Some(rgba) = picture.rgba {
        out.patches.push(OwnedPatch { id: PICTURE, offset: None, size: picture.size, rgba });
    }
    let (w, h) = (window.width(), window.height());
    let (x0, y0, width, height) = fit_rect(picture.fit, w, h);
    let [u0, v0, u1, v1] = picture.view;
    let corner = |dx: f32, dy: f32| UiVertex {
        position: [x0 + dx * width, y0 + dy * height],
        uv: [u0 + dx * (u1 - u0), v0 + dy * (v1 - v0)],
        color_rgba: [255; 4],
    };
    let mesh = UiMesh {
        vertices: vec![corner(0.0, 0.0), corner(1.0, 0.0), corner(1.0, 1.0), corner(0.0, 1.0)],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: PICTURE,
        clip: [0.0, 0.0, w, h],
    };
    let layer = out.layer.get_or_insert_with(|| UiLayer {
        compositions: Vec::new(),
        pixels_per_point: window.scale_factor(),
        meshes: Vec::new(),
    });
    layer.meshes.insert(0, mesh);
}

#[cfg(test)]
mod tests {
    use super::playback::find;
    use super::player::{Movie, Step};
    use super::*;

    /// With the install (`NFSMW_GAME_DIR`): a movie opens, its sound is as long as its picture, stepping the clock
    /// shows every frame once in order and ends; a name that is a prefix of several movies is refused.
    #[test]
    fn a_real_movie_plays_to_its_end() {
        let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
        assert!(find(&dir, "blacklist").is_err(), "several movies start with it");
        assert_eq!(find(&dir, "EALOGO").unwrap(), "ealogo_english_ntsc.vp6");
        let file = find(&dir, "ealogo").unwrap();
        let (mut movie, audio) = Movie::open(dir.read(&format!("MOVIES/{file}")).unwrap()).unwrap();
        let audio = audio.expect("the logo has sound");
        assert!(
            (-0.3..2.0).contains(&(audio.duration_secs() - movie.duration())),
            "{} vs {}",
            audio.duration_secs(),
            movie.duration()
        );
        let (mut frames, mut steps) = (0, 0);
        while movie.step(1.0 / 60.0).unwrap() != Step::Finished {
            frames += usize::from(movie.rgba().iter().any(|&b| b != 0));
            steps += 1;
            assert!(steps < 60 * 10, "the movie never ends");
        }
        assert!(frames > 20, "{frames} steps showed a picture");
    }

    /// With the install (`NFSMW_GAME_DIR`): the bounty tutorial shows both of its lines at its first cue (5.4 s), and
    /// shows nothing before it.
    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_bounty_tutorial_shows_its_first_lines_at_its_first_cue() {
        let Some(root) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
        let before = MoviePlayback::open(&dir, "bounty_tutorial", 0.0).unwrap();
        assert_eq!(before.subtitle_lines(), (None, None), "no line before the first cue");
        let at_cue = MoviePlayback::open(&dir, "bounty_tutorial", 5.4).unwrap();
        let (first, second) = at_cue.subtitle_lines();
        assert!(first.is_some() && second.is_some(), "both lines of the first cue");
    }
}

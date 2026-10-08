//! Movies (`MOVIES/*.vp6`): a scene that plays one full screen, and the plugin that shows a scene's picture.

mod player;

use anyhow::{Result, bail};
use bevy_app::{AppExit, Plugin, Update};
use bevy_ecs::prelude::*;
use bevy_window::{PrimaryWindow, Window};
use blackbox_render::{FrameParams, Instance, Renderer, UiLayer, UiMesh, UiTextureId, UiVertex};
use game_install::GameDir;
use glam::{Mat4, Vec3};

use crate::app::{FrameSet, Host};
use crate::gui::{OwnedPatch, UiOutput};
use crate::input::ActionState;
use crate::viewer::{Fit, Fullscreen, Scene};
use player::{Movie, Step};

/// Texture id of the movie picture; egui's are small numbers and the HUD's have a high bit.
const PICTURE: UiTextureId = UiTextureId(0x4D4F_5649_4500);

/// The film movies are 1024 x 512 pictures with the black bars of a wide screen baked into them: every frame of every
/// movie but the EA logo has black outside rows 64 up to 448 (measured over the first 500 frames of each movie).
const FILM_ROWS: [f32; 2] = [64.0, 448.0];
const PICTURE_ROWS: f32 = 512.0;
/// The widescreen packages show a movie in a 900 x 480 object; the logo, which has no bars, keeps that shape.
const LOGO_ASPECT: f32 = 900.0 / 480.0;

/// What to show of a movie file's frames and how: the film without its baked-in bars, stretched over the window.
fn framing(file: &str, size: [u32; 2]) -> ([f32; 4], Fit) {
    let whole = [0.0, 0.0, 1.0, 1.0];
    if file.to_ascii_lowercase().starts_with("ealogo") {
        return (whole, Fit::Contain(LOGO_ASPECT));
    }
    if size[1] != PICTURE_ROWS as u32 {
        return (whole, Fit::Contain(size[0] as f32 / size[1] as f32));
    }
    ([0.0, FILM_ROWS[0] / PICTURE_ROWS, 1.0, FILM_ROWS[1] / PICTURE_ROWS], Fit::Fill)
}

/// The movie named `name` (`ealogo`, `blacklist_03`, or the full file name), by prefix and any case.
fn find(dir: &GameDir, name: &str) -> Result<String> {
    let wanted = name.to_ascii_lowercase();
    let wanted = wanted.trim_end_matches(".vp6");
    let mut found: Vec<String> = dir
        .files_in("MOVIES")
        .into_iter()
        .filter(|f| f.to_ascii_lowercase().trim_end_matches(".vp6").starts_with(wanted))
        .collect();
    found.sort();
    match found.len() {
        0 => bail!("no movie {name:?} (try `nfsmw list-movies`)"),
        1 => Ok(found.remove(0)),
        _ => bail!("{name:?} could be {}", found.join(", ")),
    }
}

/// The movies in the install.
pub fn list(dir: &GameDir) -> Vec<String> {
    let mut movies = dir.files_in("MOVIES");
    movies.retain(|f| f.to_ascii_lowercase().ends_with(".vp6"));
    movies.sort();
    movies
}

pub struct MovieScene {
    name: String,
    movie: Movie,
    audio: Option<ea_audio::Pcm>,
    /// The latest frame, until the plugin takes it.
    frame: Option<Vec<u8>>,
    shown: bool,
    done: bool,
    /// Seconds played, and how long the sound lasts (it can outlast the picture).
    elapsed: f64,
    sound_secs: f64,
}

impl MovieScene {
    pub fn open(dir: &GameDir, name: &str, start: f64) -> Result<Self> {
        let file = find(dir, name)?;
        let bytes = anyhow::Context::with_context(dir.read(&format!("MOVIES/{file}")), || format!("reading {file}"))?;
        let (mut movie, audio) = anyhow::Context::with_context(Movie::open(bytes), || format!("opening {file}"))?;
        log::info!(
            "{file}: {:?} pixels, {:.1} s, {}",
            movie.size(),
            movie.duration(),
            audio.as_ref().map_or("no sound", |_| "with sound")
        );
        let mut frame = None;
        if start > 0.0 && movie.step(start)? == player::Step::Frame {
            frame = Some(movie.rgba().to_vec());
        }
        let sound_secs = audio.as_ref().map_or(0.0, ea_audio::Pcm::duration_secs);
        Ok(Self { name: file, movie, audio, frame, shown: false, done: false, elapsed: start, sound_secs })
    }
}

impl Scene for MovieScene {
    fn title(&self) -> String {
        format!("nfsmw — {}", self.name)
    }

    fn init(&mut self, _renderer: &mut Renderer) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, _renderer: &mut Renderer, _input: &ActionState, dt: f32) {
        self.elapsed += f64::from(dt);
        match self.movie.step(f64::from(dt)) {
            Ok(Step::Frame) => self.frame = Some(self.movie.rgba().to_vec()),
            Ok(Step::Same) => {}
            Ok(Step::Finished) => self.done = self.elapsed >= self.sound_secs,
            Err(e) => {
                log::error!("{}: {e:#}", self.name);
                self.done = true;
            }
        }
    }

    fn frame(&mut self, _aspect: f32) -> (FrameParams, &[Instance]) {
        let params = FrameParams {
            view_proj: Mat4::IDENTITY,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::NEG_Z,
            clear_color: [0.0, 0.0, 0.0],
            fog_start: f32::MAX,
            fog_end: f32::MAX,
        };
        (params, &[])
    }

    fn fullscreen(&mut self) -> Option<Fullscreen> {
        let rgba = self.frame.take();
        if rgba.is_none() && !self.shown {
            return None;
        }
        self.shown = true;
        let size = self.movie.size();
        let (view, fit) = framing(&self.name, size);
        Some(Fullscreen { size, view, fit, rgba })
    }

    fn take_clip(&mut self) -> Option<ea_audio::Pcm> {
        self.audio.take()
    }

    fn finished(&self) -> bool {
        self.done
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
    let (width, height) = match picture.fit {
        Fit::Fill => (w, h),
        Fit::Contain(aspect) if w / h > aspect => (h * aspect, h),
        Fit::Contain(aspect) => (w, w / aspect),
    };
    let (x0, y0) = ((w - width) * 0.5, (h - height) * 0.5);
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
    let layer =
        out.layer.get_or_insert_with(|| UiLayer { pixels_per_point: window.scale_factor(), meshes: Vec::new() });
    layer.meshes.insert(0, mesh);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn films_lose_their_baked_bars_and_fill_the_window_but_the_logo_keeps_its_shape() {
        let (view, fit) = framing("psa_english_ntsc.vp6", [1024, 512]);
        assert_eq!((view, fit), ([0.0, 0.125, 1.0, 0.875], Fit::Fill));
        assert_eq!(framing("EALOGO_english_ntsc.vp6", [1024, 512]), ([0.0, 0.0, 1.0, 1.0], Fit::Contain(LOGO_ASPECT)));
        let (view, fit) = framing("odd.vp6", [640, 480]);
        assert_eq!((view, fit), ([0.0, 0.0, 1.0, 1.0], Fit::Contain(640.0 / 480.0)));
    }

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
}

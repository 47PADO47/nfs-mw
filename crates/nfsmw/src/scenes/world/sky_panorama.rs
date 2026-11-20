//! The panoramic sky of a sky variant: cloud domes and a moon picture a mod ships, drawn as the clouds of the procedural sky
//! (`docs/modding/sky-variants.md`).
//!
//! The profile (`SKY/<value>/profile.toml`, `blackbox_modding::content::SkyProfile`) says which dome the sky shows at which hour
//! and cloud cover. Each frame this picks the two domes around the hour, loads them from the install's layers the first time
//! they are needed (the mod's files may be lazy: made when read), hands the renderer the pictures when they change and the blend,
//! turns and tints every frame (`SkyState::panorama`). A dome with a painted sun is turned so that the painting is under the
//! sun the game draws. Nothing here draws: the renderer's cloud map takes the domes in place of its own march.

use std::sync::Arc;

use blackbox_gfx::{PixelFormat, RenderBackend, SkyPanorama, SkyPanoramaState, TextureDesc, TextureHandle};
use blackbox_modding::content::SkyProfile;
use game_install::GameDir;

/// A picture of the profile and what became of loading it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Picture {
    #[default]
    Unloaded,
    /// The file could not be read or decoded (a warning said why): it is not tried again for this profile.
    Failed,
    Loaded(TextureHandle),
}

/// The state of the panoramic sky of the scene.
#[derive(Default)]
pub(super) struct PanoramaSky {
    profile: Option<Arc<SkyProfile>>,
    domes: Vec<Picture>,
    moon: Picture,
}

impl PanoramaSky {
    /// The profile of the sky variant in effect, or `None` for the game's own. The same profile again does nothing; another one
    /// frees the pictures of the old.
    pub(super) fn set_profile(&mut self, profile: Option<Arc<SkyProfile>>, renderer: &mut dyn RenderBackend) {
        let same = match (&self.profile, &profile) {
            (Some(old), Some(new)) => Arc::ptr_eq(old, new),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }
        self.release(renderer);
        self.domes = vec![Picture::Unloaded; profile.as_ref().map_or(0, |profile| profile.domes.len())];
        self.profile = profile;
    }

    /// Whether a sky variant is in effect.
    pub(super) fn active(&self) -> bool {
        self.profile.is_some()
    }

    /// Free the pictures: the renderer stops drawing them first.
    pub(super) fn release(&mut self, renderer: &mut dyn RenderBackend) {
        renderer.set_sky_panorama(None);
        for picture in self.domes.iter_mut().chain(std::iter::once(&mut self.moon)) {
            if let Picture::Loaded(handle) = *picture {
                renderer.destroy_texture(handle);
            }
            *picture = Picture::Unloaded;
        }
    }

    /// What the sky shows at `hour` with the cloud `cover`, with the sun at `sun_turns` (its azimuth in turns, 0 to 1, the
    /// longitude of the cloud map) and the game's cloud `scroll`. Hands the renderer the pictures; `None` when no variant is
    /// in effect or a picture the hour needs cannot be loaded (the procedural sky then draws).
    pub(super) fn frame(
        &mut self,
        renderer: &mut dyn RenderBackend,
        dir: &GameDir,
        time: (f32, f32),
        sun_turns: f32,
        scroll: f32,
    ) -> Option<SkyPanoramaState> {
        let (hour, cover) = time;
        let profile = self.profile.clone()?;
        let pose = profile.pose(hour, cover)?;
        let pictures = self.pictures(renderer, dir, &profile, pose.dome);
        renderer.set_sky_panorama(pictures);
        pictures?;
        let turns = |dome: usize| match profile.domes[dome].glow {
            Some(glow) => glow - sun_turns,
            None => scroll * profile.wind,
        };
        let moon = profile.moon.as_ref();
        Some(SkyPanoramaState {
            blend: pose.blend,
            rotation: [turns(pose.dome[0]), turns(pose.dome[1])],
            tint: pose.tint.map(|tint| tint.map(|value| value * profile.exposure)),
            gain: pose.gain,
            moon_size: moon.map_or(0.0173, |moon| moon.size),
            moon_brightness: moon.map_or(1.0, |moon| moon.brightness),
        })
    }

    /// The pictures of the two domes and the moon, loading what is not yet; `None` when a dome failed.
    fn pictures(
        &mut self,
        renderer: &mut dyn RenderBackend,
        dir: &GameDir,
        profile: &SkyProfile,
        wanted: [usize; 2],
    ) -> Option<SkyPanorama> {
        let [a, b] = wanted.map(|at| self.dome(renderer, dir, profile, at));
        let moon = match profile.moon.as_ref() {
            Some(moon) => load(&mut self.moon, renderer, dir, &moon.file),
            None => None,
        };
        Some(SkyPanorama { a: a?, b: b?, moon })
    }

    fn dome(
        &mut self,
        renderer: &mut dyn RenderBackend,
        dir: &GameDir,
        profile: &SkyProfile,
        at: usize,
    ) -> Option<TextureHandle> {
        let file = &profile.domes.get(at)?.file;
        load(self.domes.get_mut(at)?, renderer, dir, file)
    }
}

/// The texture of `file` in the install's layers, loaded the first time.
fn load(slot: &mut Picture, renderer: &mut dyn RenderBackend, dir: &GameDir, file: &str) -> Option<TextureHandle> {
    match *slot {
        Picture::Loaded(handle) => return Some(handle),
        Picture::Failed => return None,
        Picture::Unloaded => {}
    }
    *slot = match upload(renderer, dir, file) {
        Some(handle) => Picture::Loaded(handle),
        None => Picture::Failed,
    };
    match *slot {
        Picture::Loaded(handle) => Some(handle),
        _ => None,
    }
}

fn upload(renderer: &mut dyn RenderBackend, dir: &GameDir, file: &str) -> Option<TextureHandle> {
    let bytes = match dir.read(file) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::warn!("sky variant: {file} is not read: {error}; the game's own sky is drawn");
            return None;
        }
    };
    if file.to_ascii_lowercase().ends_with(".png") {
        let image = match crate::ui::decode_png_rgba(&bytes) {
            Ok(image) => image,
            Err(error) => {
                log::warn!("sky variant: {file} is not a usable PNG: {error:#}");
                return None;
            }
        };
        return Some(renderer.create_texture(&TextureDesc {
            label: file,
            width: image.width,
            height: image.height,
            format: PixelFormat::Rgba8,
            mips: vec![&image.rgba[..]],
        }));
    }
    let texture = match blackbox_tpk::read_dds(file, &bytes) {
        Ok(texture) => texture,
        Err(error) => {
            log::warn!("sky variant: {file} is not a usable DDS: {error}");
            return None;
        }
    };
    let handle = blackbox_scene::upload_texture(renderer, &texture);
    if handle.is_none() {
        log::warn!("sky variant: {file} could not be uploaded");
    }
    handle
}

#[cfg(test)]
#[path = "sky_panorama_tests.rs"]
mod tests;

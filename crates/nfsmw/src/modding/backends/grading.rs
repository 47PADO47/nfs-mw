//! Handing the colour grading variant to the scene (`docs/modding/effect-variants.md`, "Profile format of the grading").
//!
//! The choice of the `grading_variant` option is read like the other variants' (`Bridge::grading_profile`); the cubes the profile
//! names are read from the install once for each profile read ([`Bridge::grading_look`]), so what the scene is handed every
//! frame is a reference count.

use std::sync::Arc;

use super::bridge::Bridge;
use crate::grading::GradingLook;

impl Bridge {
    /// The look of the grading variant the player chose, when it is a mod's: its profile with the cubes read. A look that
    /// cannot be read is logged once and is no look (the picture is as the game draws it).
    pub fn grading_look(&self) -> Option<Arc<GradingLook>> {
        let profile = self.grading_profile();
        let mut cache = self.grading_look_cache.borrow_mut();
        let Some(profile) = profile else {
            *cache = None;
            return None;
        };
        if let Some((known, look)) = cache.as_ref()
            && Arc::ptr_eq(known, &profile)
        {
            return look.clone();
        }
        let look = self.read_grading(&profile);
        *cache = Some((profile, look.clone()));
        look
    }

    fn read_grading(&self, profile: &Arc<blackbox_modding::content::GradingProfile>) -> Option<Arc<GradingLook>> {
        let dir = self.install()?;
        match GradingLook::load(profile.as_ref().clone(), |path| dir.read(path)) {
            Ok(look) => {
                log::info!("colour grading: {} cubes read", look.cubes.len());
                Some(Arc::new(look))
            }
            Err(error) => {
                log::warn!("colour grading: the variant's cubes are not read, the picture stays as it is: {error}");
                None
            }
        }
    }
}

#[cfg(test)]
#[path = "grading_tests.rs"]
mod tests;

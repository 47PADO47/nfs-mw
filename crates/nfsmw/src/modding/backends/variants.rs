//! Effect variants: an option per kind of effect (the nitrous, the tire smoke, ...) whose only value of the game's own is
//! "Original" and to which a mod adds values (`docs/modding/effect-variants.md`).
//!
//! This is the shared part (kinds: the nitrous, the tire smoke, the collision sparks, the restored sparks). A kind is an [`EffectKind`]: the option's id and the folder of the profiles
//! (`blackbox_modding::content::VariantKind`) and the catalogue ids of its texts. The game declares one list option for each
//! kind in [`KINDS`] (`settings::builtin`), [`choice`] says which value the player chose and which mod provides it, and
//! [`Bridge::variant_profile`] reads the profile of the chosen value through the install's layers, once per chosen value, with
//! the kind's own parser. Each kind then has a few lines of its own: a cache in the bridge, a profile getter
//! (`Bridge::tire_smoke_profile`) and the scene that applies the profile (`crate::effect_variants::EffectVariants`).

use std::cell::RefCell;
use std::sync::Arc;

use blackbox_modding::content::{COLLISION_SPARKS, ContentError, NITROUS, TIRE_SMOKE, VariantKind, XENON_SPARKS};

use super::bridge::Bridge;
use super::settings::{self, ORIGINAL};

/// How many calls pass before a profile that was not found is looked for again (a mod may still be converting it).
pub(super) const PROFILE_RETRY_CALLS: u32 = 300;

/// One kind of effect that mods can add variants of.
#[derive(Debug, Clone, Copy)]
pub struct EffectKind {
    /// The option and the folder of the profiles.
    pub variant: VariantKind,
    /// What the logs call it: "nitrous", "tire smoke".
    pub name: &'static str,
    /// Catalogue message ids of the option's label and of its "Original" value.
    pub label: &'static str,
    pub original: &'static str,
}

/// The nitrous.
pub const NITROUS_KIND: EffectKind = EffectKind {
    variant: NITROUS,
    name: "nitrous",
    label: "options-nitrous-variant",
    original: "options-nitrous-variant-original",
};
/// The tire smoke.
pub const TIRE_SMOKE_KIND: EffectKind = EffectKind {
    variant: TIRE_SMOKE,
    name: "tire smoke",
    label: "options-tire-smoke-variant",
    original: "options-tire-smoke-variant-original",
};

/// The collision sparks of the Original PC spark style.
pub const COLLISION_SPARKS_KIND: EffectKind = EffectKind {
    variant: COLLISION_SPARKS,
    name: "collision spark",
    label: "options-collision-spark-variant",
    original: "options-collision-spark-variant-original",
};
/// The line sparks of the Restored (Experimental) spark style.
pub const XENON_SPARKS_KIND: EffectKind = EffectKind {
    variant: XENON_SPARKS,
    name: "restored spark",
    label: "options-xenon-spark-variant",
    original: "options-xenon-spark-variant-original",
};

/// Every kind the game offers variants of. A new kind is added here.
pub const KINDS: [EffectKind; 4] = [NITROUS_KIND, TIRE_SMOKE_KIND, COLLISION_SPARKS_KIND, XENON_SPARKS_KIND];

/// The variant of a kind in effect: the chosen value, and the mod that provides it (`None` for the game's own "Original").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantChoice {
    pub value: String,
    pub owner: Option<String>,
}

/// The variant of `kind` the player chose. Without a mod that adds one, it is always the original.
pub fn choice(bridge: &Bridge, kind: &EffectKind) -> VariantChoice {
    let option = kind.variant.option;
    let value = settings::chosen(bridge, option).unwrap_or_else(|| ORIGINAL.to_owned());
    let owner = bridge.options.borrow().value_owner(option, &value).map(str::to_owned);
    VariantChoice { value, owner }
}

/// The profile read for the chosen value of one kind, and when to look again if it was missing.
pub(super) struct ProfileCache<T> {
    value: String,
    profile: Option<Arc<T>>,
    retry_in: u32,
}

/// A bridge's cache of one kind's profile.
pub(super) type CacheCell<T> = RefCell<Option<ProfileCache<T>>>;

impl Bridge {
    /// The profile of the variant of `kind` the player chose, when it is a mod's: the file the mod keeps in its `files/` folder
    /// (`<FOLDER>/<VALUE>/profile.toml`), read through the install's layers once for each chosen value and checked with `parse`.
    /// `None` for the game's own "Original", for a variant whose mod is not loaded, and for a profile that is missing or refused
    /// (a warning says why), so the game's own effect is then exactly as it was.
    pub(super) fn variant_profile<T>(
        &self,
        kind: &EffectKind,
        cell: &CacheCell<T>,
        parse: fn(&str) -> Result<T, ContentError>,
    ) -> Option<Arc<T>> {
        let chosen = choice(self, kind);
        chosen.owner.as_ref()?;
        let mut cache = cell.borrow_mut();
        let mut first = true;
        if let Some(cached) = cache.as_mut()
            && cached.value == chosen.value
        {
            // A profile that was not there is looked for again every so often: the mod may still be converting it.
            cached.retry_in = cached.retry_in.saturating_sub(1);
            if cached.profile.is_some() || cached.retry_in > 0 {
                return cached.profile.clone();
            }
            first = false;
        }
        let dir = self.install()?;
        let (profile, found) = read_profile(&dir, kind, &chosen.value, first, parse);
        let retry_in = if found { u32::MAX } else { PROFILE_RETRY_CALLS };
        *cache = Some(ProfileCache { value: chosen.value, profile: profile.clone(), retry_in });
        profile
    }
}

/// The profile of variant `value` of `kind`, read and checked, and whether the file was there. A failure is a warning (when
/// `warn`), and the variant then has no effects.
fn read_profile<T>(
    dir: &game_install::GameDir,
    kind: &EffectKind,
    value: &str,
    warn: bool,
    parse: fn(&str) -> Result<T, ContentError>,
) -> (Option<Arc<T>>, bool) {
    let path = kind.variant.profile_path(value);
    let name = kind.name;
    let bytes = match dir.read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            if warn {
                log::warn!("the {name} variant {value:?} has no profile ({path}): {error}; the game's {name} plays");
            }
            return (None, false);
        }
    };
    let text = String::from_utf8_lossy(&bytes);
    match parse(&text) {
        Ok(profile) => (Some(Arc::new(profile)), true),
        Err(error) => {
            log::warn!("the {name} variant {value:?}: {path} is refused: {error}; the game's {name} plays");
            (None, true)
        }
    }
}

#[cfg(test)]
#[path = "variants_tests.rs"]
mod tests;

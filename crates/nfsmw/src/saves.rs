//! The game's save slots: a payload written to a `save-store` slot, in this game's envelope. The envelope is the MD5
//! digest of the rest of the file, then the `"Game"` magic, the version word, and the payload. The envelope is our own
//! format, not the original's (docs/specs/save-alias-manager.md, section 10). Nothing calls this yet; the save screens
//! that would are not built.

// No caller yet: the save screens that would use this are not built (docs/specs/save-alias-manager.md).
#![allow(dead_code)]

use save_store::digest;
use save_store::{Aliases, SaveStore, StoreError};

use crate::save_format::{self, HeaderError, MAGIC};
use crate::settings::{Partial, Percent, Settings};

/// The version this build writes. A later build reads older versions by checking the word, not by guessing.
pub const VERSION: u32 = 8;

#[derive(Debug)]
pub enum SaveError {
    Store(StoreError),
    Digest(digest::DigestError),
    Header(HeaderError),
    /// The slot has not been written.
    Missing,
    /// The payload is not what the caller expected (an alias index that does not parse, say).
    Payload(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(e) => write!(f, "{e}"),
            Self::Digest(e) => write!(f, "{e}"),
            Self::Header(e) => write!(f, "{e}"),
            Self::Missing => write!(f, "the save slot has not been written"),
            Self::Payload(e) => write!(f, "the save payload is not readable: {e}"),
        }
    }
}

impl From<StoreError> for SaveError {
    fn from(e: StoreError) -> Self {
        Self::Store(e)
    }
}

/// The envelope around a payload: the magic, the version, then the payload. The digest is added by [`digest::with_prefix`].
pub fn encode(payload: &[u8]) -> Vec<u8> {
    let mut body = MAGIC.to_le_bytes().to_vec();
    body.extend_from_slice(&VERSION.to_le_bytes());
    body.extend_from_slice(payload);
    digest::with_prefix(&body)
}

/// The payload of a file in this envelope, after the digest and the header are checked.
pub fn decode(file: &[u8]) -> Result<&[u8], SaveError> {
    let body = digest::check_prefixed(file).map_err(SaveError::Digest)?;
    save_format::check_header(body).map_err(SaveError::Header)?;
    body.get(8..).ok_or(SaveError::Header(HeaderError::TooShort))
}

/// Writes a payload to a slot.
pub fn save(store: &SaveStore, slot: &str, payload: &[u8]) -> Result<(), SaveError> {
    store.write(slot, &encode(payload)).map_err(SaveError::from)
}

/// Reads a slot's payload, checking its envelope.
pub fn load(store: &SaveStore, slot: &str) -> Result<Vec<u8>, SaveError> {
    let file = store.read(slot)?.ok_or(SaveError::Missing)?;
    decode(&file).map(<[u8]>::to_vec)
}

/// The alias index as a payload: its TOML text, one `alias = "slot"` line per alias.
pub fn aliases_payload(aliases: &Aliases) -> Result<Vec<u8>, SaveError> {
    let table: toml::Table = aliases.iter().map(|(a, s)| (a.to_owned(), toml::Value::String(s.to_owned()))).collect();
    toml::to_string(&table).map(String::into_bytes).map_err(|e| SaveError::Payload(e.to_string()))
}

/// The alias index read back from a payload. A payload that does not parse, or names an invalid slot, is refused.
pub fn aliases_from_payload(payload: &[u8]) -> Result<Aliases, SaveError> {
    let text = std::str::from_utf8(payload).map_err(|e| SaveError::Payload(e.to_string()))?;
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| SaveError::Payload(e.to_string()))?;
    let mut aliases = Aliases::default();
    for (alias, value) in table {
        let toml::Value::String(slot) = value else {
            return Err(SaveError::Payload(format!("alias {alias:?} does not name a slot")));
        };
        aliases.assign(&alias, &slot)?;
    }
    Ok(aliases)
}

/// Saves the alias index to a slot.
pub fn save_aliases(store: &SaveStore, slot: &str, aliases: &Aliases) -> Result<(), SaveError> {
    save(store, slot, &aliases_payload(aliases)?)
}

/// Loads the alias index from a slot.
pub fn load_aliases(store: &SaveStore, slot: &str) -> Result<Aliases, SaveError> {
    aliases_from_payload(&load(store, slot)?)
}

/// The slot the alias index is saved in, and the alias the game's own save uses by default.
pub const ALIAS_INDEX_SLOT: &str = "aliases";
/// A new alias is named `Save N` and kept in the slot `save-N` (`save_new_alias`).
const SAVE_PREFIX: &str = "Save ";
const SAVE_SLOT_PREFIX: &str = "save-";

/// The save folder: `saves` next to the config file. `None` when there is no per-user config folder.
pub fn store() -> Option<SaveStore> {
    let config = crate::settings::Settings::config_path()?;
    Some(SaveStore::new(config.parent()?.join("saves")))
}

/// Every setting as a `Partial`, so the whole set can be written to the config's TOML text.
pub fn partial_of(s: &Settings) -> Partial {
    let c = &s.controls;
    Partial {
        traffic_policy: None,
        pursuit_options: Some(s.pursuit_options),
        detail: None,
        appearance: Some(s.appearance),
        deadzone_mode: Some(c.deadzone_mode),
        steering_deadzone: Some(c.steering_deadzone),
        camera_deadzone: Some(c.camera_deadzone),
        trigger_deadzone: Some(c.trigger_deadzone),
        steering_sensitivity: Some(c.steering_sensitivity),
        camera_sensitivity: Some(c.camera_sensitivity),
        mouse_sensitivity: Some(c.mouse_sensitivity),
        invert_camera_y: Some(c.invert_camera_y),
        backend: Some(s.backend),
        renderer: Some(s.renderer),
        reflex: Some(s.reflex),
        frame_generation: Some(s.frame_generation),
        vsync: Some(s.vsync),
        max_fps: Some(s.max_fps),
        show_metrics: Some(s.show_metrics),
        window_mode: Some(s.window_mode),
        monitor: Some(s.monitor),
        resolution: Some(s.resolution),
        show_readout: Some(s.show_readout),
        master_volume: Some(s.master_volume),
        music_volume: Some(s.music_volume),
        sfx_volume: Some(s.sfx_volume),
        engine_volume: Some(s.engine_volume),
        speech_volume: Some(s.speech_volume),
        hud: Some(s.hud),
        tire_smoke: Some(s.tire_smoke),
        radio: Some(s.radio),
        skip_intro: Some(s.skip_intro),
        graphics_preset: Some(s.graphics_preset),
        shadows: Some(s.shadows),
        car_shading: Some(s.car_shading),
        rain_droplet_scale: Some(s.rain_droplet_scale),
        road_reflections: Some(s.road_reflections),
        road_reflection_resolution: Some(s.road_reflection_resolution),
        traffic: Some(s.traffic),
        cop_share: Some(s.cop_share),
        always_rain: Some(s.always_rain),
        motion_blur: Some(s.motion_blur),
        motion_blur_strength: Some(s.motion_blur_strength),
        hd_content: s.hd_content.into(),
        custom_fonts: s.custom_fonts.into(),
        fe_shadows: s.fe_shadows.into(),
        net: s.net.into(),
        rain_amount: Some(s.rain_amount),
        game_speed: Some(s.game_speed),
        animation_speed: Some(s.animation_speed),
        car_scale: Some(s.car_scale),
        no_rev_limiter: Some(s.no_rev_limiter),
        show_hud_on_look_back: Some(s.show_hud_on_look_back),
        traffic_lights: Some(s.traffic_lights),
        traffic_avoidance: Some(s.traffic_avoidance),
        cop_health_bars: Some(s.cop_health_bars),
        smoke_quality: Some(s.smoke_quality),
        skid_marks: Some(s.skid_marks),
        collision_sparks: Some(s.collision_sparks),
        spark_style: Some(s.spark_style),
        speed_trails: Some(s.speed_trails),
        speed_trail_style: Some(s.speed_trail_style),
        exhaust_flames: Some(s.exhaust_flames),
        turn_signals: Some(s.turn_signals),
        rear_view_mirror: Some(s.rear_view_mirror),
        discord: Some(s.discord),
        discord_time_played: Some(s.discord_time_played),
        mods: None,
        transmission: Some(s.transmission),
        camera_mode: Some(s.camera_mode),
        minimap: Some(s.minimap),
        hud_layout: Some(s.hud_layout),
        ui_style: Some(s.ui_style),
        speed_units: Some(s.speed_units),
        ui_theme: Some(s.ui_theme),
        post_tonemap: Some(s.post_tonemap),
        post_bloom: Some(s.post_bloom),
        post_aa: Some(s.post_aa),
        texture_filtering: Some(s.texture_filtering),
        visual_treatment: Some(s.visual_treatment),
        reflections: Some(s.reflections),
        reflection_cube_size: Some(s.reflection_cube_size),
        reflection_mirror_size: Some(s.reflection_mirror_size),
        reflection_pow2: Some(s.reflection_pow2),
        mirror_tint: Some(s.mirror_tint),
        reflection_distance: Some(s.reflection_extra.distance),
        reflection_lod: Some(s.reflection_extra.lod),
        reflection_content: Some(s.reflection_extra.content),
        vehicle_reflection_brightness: Some(s.reflection_extra.brightness),
        ao: Some(s.ao),
        ao_intensity: Some(s.ao_intensity),
        ao_compare: Some(s.ao_compare),
        ao_depth_mips: Some(s.ao_depth_mips),
        ao_method: Some(s.ao_method),
        render_scale: Some(s.render_scale),
        upscaler: Some(s.upscaler),
        upscale_sharpness: Some(s.upscale_sharpness),
        upscale_quality: Some(s.upscale_quality),
        ray_tracing: Some(s.ray_tracing),
        paddle_up: s.paddle_up,
        headlights_key: s.headlights_key,
        paddle_down: s.paddle_down,
        manual_clutch: Some(s.manual_clutch),
        h_shifter: Some(s.h_shifter),
    }
}

/// The settings the config's TOML file does not carry (they live in the extra-options layer): written as their own
/// keys in the alias payload, so an alias keeps every setting.
fn extra_table(s: &Settings) -> toml::Table {
    let mut t = toml::Table::new();
    t.insert("skip_intro".into(), toml::Value::Boolean(s.skip_intro));
    t.insert("always_rain".into(), toml::Value::Boolean(s.always_rain));
    t.insert("rain_amount".into(), toml::Value::Integer(i64::from(s.rain_amount.0)));
    t.insert("game_speed".into(), toml::Value::Float(f64::from(s.game_speed)));
    t.insert("animation_speed".into(), toml::Value::Float(f64::from(s.animation_speed)));
    t.insert("car_scale".into(), toml::Value::Float(f64::from(s.car_scale)));
    t.insert("no_rev_limiter".into(), toml::Value::Boolean(s.no_rev_limiter));
    t.insert("show_hud_on_look_back".into(), toml::Value::Boolean(s.show_hud_on_look_back));
    if let Some(key) = s.headlights_key {
        t.insert("headlights_key".into(), toml::Value::Integer(i64::from(key)));
    }
    t
}

/// The settings as TOML text, the payload of an alias: the extra keys, then the config's keys. The extra keys
/// come first because the config can end in a table (`[hd_content]`), which would otherwise swallow them.
pub fn settings_payload(s: &Settings) -> Result<Vec<u8>, SaveError> {
    let config = crate::settings::write::merge("", &partial_of(s)).map_err(|e| SaveError::Payload(format!("{e:#}")))?;
    let mut table: toml::Table = config.parse().map_err(|e: toml::de::Error| SaveError::Payload(e.to_string()))?;
    table.extend(extra_table(s));
    let text = toml::to_string(&table).map_err(|e| SaveError::Payload(e.to_string()))?;
    Ok(text.into_bytes())
}

/// The settings read back from a payload. A key the payload lacks takes its default.
pub fn settings_from_payload(payload: &[u8]) -> Result<Settings, SaveError> {
    let text = std::str::from_utf8(payload).map_err(|e| SaveError::Payload(e.to_string()))?;
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| SaveError::Payload(e.to_string()))?;
    let mut partial = crate::settings::file::parse(text, "save");
    let flag = |key: &str| table.get(key).and_then(toml::Value::as_bool);
    let number = |key: &str| table.get(key).and_then(toml::Value::as_float).map(|f| f as f32);
    partial.skip_intro = flag("skip_intro");
    partial.always_rain = flag("always_rain");
    partial.rain_amount =
        table.get("rain_amount").and_then(toml::Value::as_integer).and_then(|i| u8::try_from(i).ok()).map(Percent);
    partial.game_speed = number("game_speed");
    partial.animation_speed = number("animation_speed");
    partial.car_scale = number("car_scale");
    partial.no_rev_limiter = flag("no_rev_limiter");
    partial.show_hud_on_look_back = flag("show_hud_on_look_back");
    partial.headlights_key =
        table.get("headlights_key").and_then(toml::Value::as_integer).and_then(|i| u32::try_from(i).ok());
    Ok(Settings::from(partial))
}

/// The alias index, or an empty one when none has been saved yet.
fn index_or_empty(store: &SaveStore) -> Result<Aliases, SaveError> {
    match store.exists(ALIAS_INDEX_SLOT)? {
        true => load_aliases(store, ALIAS_INDEX_SLOT),
        false => Ok(Aliases::default()),
    }
}

/// The number of an alias named `Save N`, or `None` for any other name.
fn save_number(alias: &str) -> Option<u32> {
    alias.strip_prefix(SAVE_PREFIX)?.parse().ok()
}

/// Saves the settings as a new alias, named `Save N` with the first N that is free (both its name and its slot). The
/// slot is `save-N`. Returns the alias's name.
pub fn save_new_alias(store: &SaveStore, settings: &Settings) -> Result<String, SaveError> {
    let mut aliases = index_or_empty(store)?;
    let number = (1u32..)
        .find(|n| {
            let slot = format!("{SAVE_SLOT_PREFIX}{n}");
            aliases.slot(&format!("{SAVE_PREFIX}{n}")).is_none()
                && !aliases.iter().any(|(_, s)| s == slot)
                && !store.exists(&slot).unwrap_or(true)
        })
        .unwrap_or(u32::MAX);
    let name = format!("{SAVE_PREFIX}{number}");
    let slot = format!("{SAVE_SLOT_PREFIX}{number}");
    save(store, &slot, &settings_payload(settings)?)?;
    aliases.assign(&name, &slot)?;
    save_aliases(store, ALIAS_INDEX_SLOT, &aliases)?;
    Ok(name)
}

/// The most recently saved alias (the highest `Save N`) and its settings, or `None` when there is none.
pub fn load_latest_alias(store: &SaveStore) -> Result<Option<(String, Settings)>, SaveError> {
    if !store.exists(ALIAS_INDEX_SLOT)? {
        return Ok(None);
    }
    let aliases = load_aliases(store, ALIAS_INDEX_SLOT)?;
    let latest = aliases.iter().filter_map(|(name, slot)| Some((save_number(name)?, name, slot))).max();
    let Some((_, name, slot)) = latest else { return Ok(None) };
    let settings = settings_from_payload(&load(store, slot)?)?;
    Ok(Some((name.to_owned(), settings)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn an_alias_index_round_trips_through_a_slot() {
        let dir = scratch("aliases");
        let _ = fs::remove_dir_all(&dir);
        let store = SaveStore::new(&dir);
        let mut aliases = Aliases::default();
        aliases.assign("My Career", "career_1").unwrap();
        aliases.assign("Test", "slot-2").unwrap();
        save_aliases(&store, "index", &aliases).unwrap();
        let back = load_aliases(&store, "index").unwrap();
        assert_eq!(back.slot("My Career"), Some("career_1"));
        assert_eq!(back.iter().count(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_alias_keeps_every_setting() {
        let dir = scratch("settings");
        let _ = fs::remove_dir_all(&dir);
        let store = SaveStore::new(&dir);
        assert!(load_latest_alias(&store).unwrap().is_none(), "no alias before the first save");
        let mut settings = Settings::from(Partial::default());
        settings.controls.steering_deadzone = settings.controls.steering_deadzone.step(true);
        settings.game_speed = 0.5;
        settings.traffic = 7;
        settings.ao_compare = !settings.ao_compare;
        settings.ao_depth_mips = !settings.ao_depth_mips;
        settings.paddle_up = Some(0x1234);
        let name = save_new_alias(&store, &settings).unwrap();
        assert_eq!(load_latest_alias(&store).unwrap(), Some((name, settings)));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn each_save_is_its_own_named_alias_and_load_takes_the_latest() {
        let dir = scratch("named");
        let _ = fs::remove_dir_all(&dir);
        let store = SaveStore::new(&dir);
        let mut first = Settings::from(Partial::default());
        first.traffic = 1;
        let mut second = first;
        second.traffic = 2;
        assert_eq!(save_new_alias(&store, &first).unwrap(), "Save 1");
        assert_eq!(save_new_alias(&store, &second).unwrap(), "Save 2");
        let aliases = load_aliases(&store, ALIAS_INDEX_SLOT).unwrap();
        assert_eq!(aliases.slot("Save 1"), Some("save-1"));
        assert_eq!(aliases.slot("Save 2"), Some("save-2"));
        assert_eq!(load_latest_alias(&store).unwrap(), Some(("Save 2".to_owned(), second)));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_payload_that_is_not_an_alias_index_is_refused() {
        assert!(matches!(aliases_from_payload(b"x = 3"), Err(SaveError::Payload(_))));
        assert!(matches!(aliases_from_payload(&[0xff]), Err(SaveError::Payload(_))));
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nfsmw-saves-{name}-{}", std::process::id()))
    }

    #[test]
    fn a_payload_round_trips_through_a_slot() {
        let dir = scratch("round");
        let _ = fs::remove_dir_all(&dir);
        let store = SaveStore::new(&dir);
        save(&store, "career", b"payload").unwrap();
        assert_eq!(load(&store, "career").unwrap(), b"payload");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_slot_says_so() {
        let dir = scratch("missing");
        let _ = fs::remove_dir_all(&dir);
        assert!(matches!(load(&SaveStore::new(&dir), "none"), Err(SaveError::Missing)));
    }

    #[test]
    fn a_changed_byte_is_refused() {
        let mut file = encode(b"payload");
        *file.last_mut().unwrap() ^= 1;
        assert!(matches!(decode(&file), Err(SaveError::Digest(_))));
    }

    #[test]
    fn a_file_with_a_valid_digest_but_no_magic_is_refused() {
        let file = digest::with_prefix(b"\0\0\0\0\0\0\0\0payload");
        assert!(matches!(decode(&file), Err(SaveError::Header(HeaderError::BadMagic))));
    }
}

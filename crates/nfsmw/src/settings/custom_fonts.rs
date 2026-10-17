//! Custom fonts: FEng fonts a player supplies in a folder, one set per resolution tier. Off by default, since the
//! fonts change every text screen. The rules are in `docs/specs/custom-fonts.md`; the tiers are in `ui/fonts/tier.rs`.
//!
//! Config: `[custom_fonts]` with `enabled`, `tier` (`auto`, `high`, `mid` or `low`), `scale.high`, `scale.mid` and
//! `scale.low`, and `folder` (a path, relative to the config folder; the default is `fonts` there, read by [`folder`]).
//! Environment: `NFSMW_CUSTOM_FONTS`. Command line: `--custom-fonts` / `--no-custom-fonts`. Console: `set custom_fonts on`.

use std::path::PathBuf;
use std::sync::RwLock;

use bevy_ecs::prelude::{Query, Res, With};
use bevy_window::{PrimaryWindow, Window};
use clap::Args;
use toml::{Table, Value};

use super::Settings;
use super::partial::parse_bool;
use crate::ui::fonts::tier::{self, Tier, TierSetting};

/// Custom fonts are off: they replace the fonts of every text screen.
pub const DEFAULT_ENABLED: bool = false;
/// A tier's scale is 1.0 unless the config sets it: the custom font is drawn at its own size.
pub const DEFAULT_SCALE: f32 = 1.0;
/// The scales a config may set: above 0 and at most this.
pub const MAX_SCALE: f32 = 4.0;

/// The environment variable of the switch.
pub const ENV: &str = "NFSMW_CUSTOM_FONTS";
/// The console key of the switch.
pub const KEY: &str = "custom_fonts";
/// The font folder, relative to the config folder, when the config does not name one.
pub const FOLDER: &str = "fonts";

/// The options as the settings hold them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CustomFonts {
    /// The custom fonts replace the stock fonts that have a file (`enabled`).
    pub enabled: bool,
    /// The tier: from the render height, or pinned (`tier`).
    pub tier: TierSetting,
    /// The scale of a custom font drawn from each tier's file, indexed by [`Tier::index`] (`scale.high`, `scale.mid`,
    /// `scale.low`).
    pub scale: [f32; 3],
}

impl Default for CustomFonts {
    fn default() -> Self {
        Self { enabled: DEFAULT_ENABLED, tier: TierSetting::Auto, scale: [DEFAULT_SCALE; 3] }
    }
}

impl CustomFonts {
    /// The values of a merged layer, with the defaults where no layer set one.
    pub fn from_layer(layer: CustomFontsLayer) -> Self {
        Self {
            enabled: layer.enabled.unwrap_or(DEFAULT_ENABLED),
            tier: layer.tier.unwrap_or_default(),
            scale: layer.scale.map(|s| s.unwrap_or(DEFAULT_SCALE)),
        }
    }
}

/// One layer of the options: what a config file, the environment or the command line set.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CustomFontsLayer {
    pub enabled: Option<bool>,
    pub tier: Option<TierSetting>,
    pub scale: [Option<f32>; 3],
}

impl CustomFontsLayer {
    /// This layer, with `lower` filling in what it leaves unset.
    pub fn or(self, lower: Self) -> Self {
        Self {
            enabled: self.enabled.or(lower.enabled),
            tier: self.tier.or(lower.tier),
            scale: [0, 1, 2].map(|i| self.scale[i].or(lower.scale[i])),
        }
    }
}

impl From<CustomFonts> for CustomFontsLayer {
    /// Every option set, for writing the whole set of values to the config file.
    fn from(value: CustomFonts) -> Self {
        Self { enabled: Some(value.enabled), tier: Some(value.tier), scale: value.scale.map(Some) }
    }
}

/// What the UI reads each frame: the switch, the tier in use and the scales, with the tier taken from the render
/// height. Kept in step by [`publish`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Active {
    pub enabled: bool,
    pub tier: Tier,
    pub scale: [f32; 3],
}

impl Active {
    /// The scale of a custom font drawn from `tier`'s file.
    pub fn scale_of(&self, tier: Tier) -> f32 {
        self.scale[tier.index()]
    }
}

const OFF: Active = Active { enabled: false, tier: Tier::Low, scale: [DEFAULT_SCALE; 3] };

/// The state in force now (see [`Active`]).
static ACTIVE: RwLock<Active> = RwLock::new(OFF);

/// The custom font state in force now.
pub fn active() -> Active {
    match ACTIVE.read() {
        Ok(value) => *value,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

/// Copies the options and the tier for the window's render height to [`active`]. Runs every frame, so a console change
/// or a window resize takes effect at once. Without a window it keeps the last values.
pub fn publish(settings: Res<Settings>, windows: Query<&Window, With<PrimaryWindow>>) {
    let Ok(window) = windows.single() else { return };
    let value = active_for(&settings.custom_fonts, window.physical_height());
    let Ok(mut active) = ACTIVE.write() else { return };
    *active = value;
}

/// The state for the options and a render height.
pub fn active_for(options: &CustomFonts, height: u32) -> Active {
    Active { enabled: options.enabled, tier: tier::select(options.tier, height), scale: options.scale }
}

/// The font folder: the `folder` of the config's `[custom_fonts]` table (a path relative to the config folder, or
/// absolute), else `fonts` in the config folder. `None` when the platform has no config folder.
pub fn folder() -> Option<PathBuf> {
    let config = Settings::config_path()?;
    let dir = config.parent()?.to_path_buf();
    let text = std::fs::read_to_string(&config).unwrap_or_default();
    let table: Table = text.parse().unwrap_or_default();
    let named = table
        .get(KEY)
        .and_then(Value::as_table)
        .and_then(|section| section.get("folder"))
        .and_then(Value::as_str)
        .map(PathBuf::from);
    Some(dir.join(named.unwrap_or_else(|| PathBuf::from(FOLDER))))
}

/// The `[custom_fonts]` table of the config file. A value of the wrong kind, or a scale outside (0, 4], is reported and
/// left unset.
pub fn read(table: &Table) -> CustomFontsLayer {
    let Some(section) = table.get(KEY).and_then(Value::as_table) else {
        return CustomFontsLayer::default();
    };
    let scales = section.get("scale").and_then(Value::as_table);
    CustomFontsLayer {
        enabled: config_bool(section, "enabled"),
        tier: config_tier(section),
        scale: Tier::ALL.map(|tier| scales.and_then(|t| config_scale(t, tier.name()))),
    }
}

fn config_bool(section: &Table, name: &str) -> Option<bool> {
    let value = section.get(name)?;
    let parsed = value.as_bool();
    if parsed.is_none() {
        log::warn!("config [custom_fonts] {name}: expected true or false");
    }
    parsed
}

fn config_tier(section: &Table) -> Option<TierSetting> {
    let value = section.get("tier")?;
    let parsed = TierSetting::from_name(value.as_str().unwrap_or_default());
    parsed.map_err(|e| log::warn!("config [custom_fonts] tier: {e}")).ok()
}

fn config_scale(scales: &Table, name: &str) -> Option<f32> {
    let value = scales.get(name)?;
    let Some(number) = value.as_float().or_else(|| value.as_integer().map(|i| i as f64)) else {
        log::warn!("config [custom_fonts] scale.{name}: expected a number");
        return None;
    };
    let scale = number as f32;
    if !is_valid_scale(scale) {
        log::warn!("config [custom_fonts] scale.{name}: {number} is outside (0, {MAX_SCALE}]; using {DEFAULT_SCALE}");
        return None;
    }
    Some(scale)
}

/// Whether a scale is above 0 and at most [`MAX_SCALE`]. NaN is not.
pub fn is_valid_scale(scale: f32) -> bool {
    scale > 0.0 && scale <= MAX_SCALE
}

/// The environment variable. A value that does not parse is ignored.
pub fn from_env(get: impl Fn(&str) -> Option<String>) -> CustomFontsLayer {
    CustomFontsLayer { enabled: get(ENV).and_then(|text| parse_bool(&text).ok()), ..Default::default() }
}

/// Puts the options `changes` sets into the `[custom_fonts]` table of `table`, keeping the rest of the file. A table
/// that is not a table is left alone rather than overwritten.
pub fn merge_into(table: &mut Table, changes: &CustomFontsLayer) {
    if *changes == CustomFontsLayer::default() {
        return;
    }
    let entry = table.entry(KEY).or_insert_with(|| Value::Table(Table::new()));
    let Some(section) = entry.as_table_mut() else { return };
    if let Some(enabled) = changes.enabled {
        section.insert("enabled".to_owned(), Value::Boolean(enabled));
    }
    if let Some(tier) = changes.tier {
        section.insert("tier".to_owned(), Value::String(tier.name().to_owned()));
    }
    if changes.scale.iter().all(Option::is_none) {
        return;
    }
    let scales = section.entry("scale").or_insert_with(|| Value::Table(Table::new()));
    let Some(scales) = scales.as_table_mut() else { return };
    for (tier, value) in Tier::ALL.into_iter().zip(changes.scale) {
        if let Some(scale) = value {
            scales.insert(tier.name().to_owned(), Value::Float(f64::from(scale)));
        }
    }
}

/// The command-line switch. Unset switches leave the config file or the environment in charge.
#[derive(Debug, Clone, Default, Args)]
pub struct CustomFontsArgs {
    /// Replace the stock fonts with the files in the custom font folder [env NFSMW_CUSTOM_FONTS; config `[custom_fonts] enabled`; default off].
    #[arg(long, conflicts_with = "no_custom_fonts")]
    pub custom_fonts: bool,
    /// Keep the stock fonts.
    #[arg(long)]
    pub no_custom_fonts: bool,
}

impl CustomFontsArgs {
    pub fn layer(&self) -> CustomFontsLayer {
        let enabled = match (self.custom_fonts, self.no_custom_fonts) {
            (true, _) => Some(true),
            (false, true) => Some(false),
            (false, false) => None,
        };
        CustomFontsLayer { enabled, ..Default::default() }
    }
}

/// Whether the console key names the option.
pub fn is_key(key: &str) -> bool {
    key == KEY
}

/// The switch a console key names, for a switch with no value.
pub fn flag<'a>(settings: &'a mut CustomFonts, key: &str) -> Option<&'a mut bool> {
    (key == KEY).then_some(&mut settings.enabled)
}

/// The value of the option as the console shows it.
pub fn get(settings: &CustomFonts, _key: &str) -> String {
    let text = match settings.enabled {
        true => "on",
        false => "off",
    };
    text.to_owned()
}

/// Sets the option from a console value (`on` or `off`).
pub fn set(settings: &mut CustomFonts, key: &str, value: &str) -> Result<(), String> {
    let Some(option) = flag(settings, key) else { return Err(format!("unknown setting {key}")) };
    *option = parse_bool(value)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_feature_is_off_with_the_stock_fonts_and_auto_tiers() {
        let s = CustomFonts::default();
        assert_eq!(s, CustomFonts { enabled: false, tier: TierSetting::Auto, scale: [1.0; 3] });
        assert_eq!(CustomFonts::from_layer(CustomFontsLayer::default()), s);
    }

    #[test]
    fn the_higher_layer_wins_and_gaps_fall_through() {
        let high = CustomFontsLayer { enabled: Some(true), scale: [Some(1.25), None, None], ..Default::default() };
        let low =
            CustomFontsLayer { enabled: Some(false), tier: Some(TierSetting::Auto), scale: [None, Some(0.5), None] };
        let merged = high.or(low);
        assert_eq!(merged.enabled, Some(true));
        assert_eq!(merged.scale, [Some(1.25), Some(0.5), None]);
        let s = CustomFonts::from_layer(merged);
        assert_eq!((s.enabled, s.scale), (true, [1.25, 0.5, 1.0]));
    }

    #[test]
    fn the_config_table_is_read_and_bad_values_are_left_unset() {
        let table: Table =
            "[custom_fonts]\nenabled = true\ntier = \"mid\"\n[custom_fonts.scale]\nhigh = 1.25\nmid = 0\nlow = 9\n"
                .parse()
                .unwrap();
        let layer = read(&table);
        assert_eq!(layer.enabled, Some(true));
        assert_eq!(layer.tier, Some(TierSetting::Pinned(Tier::Mid)));
        assert_eq!(layer.scale, [Some(1.25), None, None]);
        let bad: Table = "[custom_fonts]\ntier = \"ultra\"\nenabled = \"yes\"\n".parse().unwrap();
        assert_eq!(read(&bad), CustomFontsLayer::default());
        let none: Table = "vsync = true\n".parse().unwrap();
        assert_eq!(read(&none), CustomFontsLayer::default());
    }

    #[test]
    fn the_environment_and_the_switches_set_the_switch() {
        let env = |name: &str| (name == ENV).then(|| "on".to_owned());
        assert_eq!(from_env(env).enabled, Some(true));
        assert_eq!(from_env(|_| Some("maybe".to_owned())).enabled, None);
        let on = CustomFontsArgs { custom_fonts: true, ..Default::default() };
        assert_eq!(on.layer().enabled, Some(true));
        let off = CustomFontsArgs { no_custom_fonts: true, ..Default::default() };
        assert_eq!(off.layer().enabled, Some(false));
        assert_eq!(CustomFontsArgs::default().layer(), CustomFontsLayer::default());
    }

    #[test]
    fn merging_keeps_the_other_keys_of_the_file() {
        let mut table: Table = "vsync = true\n[custom_fonts]\ntier = \"low\"\n".parse().unwrap();
        let changes = CustomFontsLayer { enabled: Some(true), tier: None, scale: [Some(1.5), None, None] };
        merge_into(&mut table, &changes);
        let section = table[KEY].as_table().unwrap();
        assert_eq!(section["enabled"].as_bool(), Some(true));
        assert_eq!(section["tier"].as_str(), Some("low"));
        assert_eq!(section["scale"].as_table().unwrap()["high"].as_float(), Some(1.5));
        assert_eq!(table["vsync"].as_bool(), Some(true));
    }

    #[test]
    fn the_console_toggles_the_switch() {
        let mut s = CustomFonts::default();
        assert!(is_key("custom_fonts") && !is_key("custom_fonts_tier"));
        assert_eq!(set(&mut s, "custom_fonts", "on"), Ok(()));
        assert!(s.enabled);
        assert_eq!(get(&s, "custom_fonts"), "on");
        assert!(set(&mut s, "custom_fonts", "sometimes").is_err());
        assert!(set(&mut s, "custom_fonts_other", "on").is_err());
        *flag(&mut s, "custom_fonts").unwrap() = false;
        assert!(!s.enabled);
    }

    #[test]
    fn the_tier_in_use_comes_from_the_height_unless_pinned() {
        let auto = CustomFonts { enabled: true, ..Default::default() };
        assert_eq!(active_for(&auto, 1440).tier, Tier::High);
        assert_eq!(active_for(&auto, 1080).tier, Tier::Mid);
        assert_eq!(active_for(&auto, 720).tier, Tier::Low);
        let pinned = CustomFonts { tier: TierSetting::Pinned(Tier::Low), scale: [1.0, 1.0, 2.0], ..auto };
        let active = active_for(&pinned, 1440);
        assert_eq!((active.tier, active.enabled), (Tier::Low, true));
        assert_eq!(active.scale_of(Tier::Low), 2.0);
        assert!(is_valid_scale(4.0) && !is_valid_scale(0.0) && !is_valid_scale(f32::NAN) && !is_valid_scale(4.5));
    }
}

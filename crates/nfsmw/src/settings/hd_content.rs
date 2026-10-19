//! The HD content options: HD font glyph sizes, the HD pointer hotspot and HD movie framing. Each corrects a bug that
//! shows with replacement (HD) content and changes nothing for stock content, so each is on by default. The rules are in
//! `docs/specs/hd-content-support.md`.
//!
//! Config: `[hd_content]` with `fonts`, `cursor` and `fmv`. Environment: `NFSMW_HD_FONTS`, `NFSMW_HD_CURSOR`,
//! `NFSMW_HD_FMV`. Command line: `--hd-fonts` / `--no-hd-fonts` and the same for the others. Console: `set hd_fonts off`.

use std::sync::RwLock;

use bevy_ecs::prelude::Res;
use clap::Args;
use toml::{Table, Value};

use super::Settings;
use super::partial::parse_bool;

/// HD font glyphs keep the stock texture's layout.
pub const DEFAULT_FONTS: bool = true;
/// The pointer hotspot follows the size of an HD cursor texture.
pub const DEFAULT_CURSOR: bool = true;
/// Film movies keep the stock 1024 x 512 framing whatever their size.
pub const DEFAULT_FMV: bool = true;

pub const FONTS_ENV: &str = "NFSMW_HD_FONTS";
pub const CURSOR_ENV: &str = "NFSMW_HD_CURSOR";
pub const FMV_ENV: &str = "NFSMW_HD_FMV";

/// The console keys of the three options.
pub const KEYS: [&str; 3] = ["hd_fonts", "hd_cursor", "hd_fmv"];

const ON: HdContent = HdContent { fonts: DEFAULT_FONTS, cursor: DEFAULT_CURSOR, fmv: DEFAULT_FMV };

/// The options as the settings hold them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdContent {
    /// Font glyphs are placed by the stock texture's size, not the loaded one (`fonts`).
    pub fonts: bool,
    /// The pointer's hotspot scales with the cursor texture (`cursor`).
    pub cursor: bool,
    /// Film movies are framed as the stock 1024 x 512 pictures (`fmv`).
    pub fmv: bool,
}

impl Default for HdContent {
    fn default() -> Self {
        ON
    }
}

impl HdContent {
    /// The values of a merged layer, with the defaults where no layer set one.
    pub fn from_layer(layer: HdContentLayer) -> Self {
        Self {
            fonts: layer.fonts.unwrap_or(DEFAULT_FONTS),
            cursor: layer.cursor.unwrap_or(DEFAULT_CURSOR),
            fmv: layer.fmv.unwrap_or(DEFAULT_FMV),
        }
    }
}

/// One layer of the options: what a config file, the environment or the command line set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HdContentLayer {
    pub fonts: Option<bool>,
    pub cursor: Option<bool>,
    pub fmv: Option<bool>,
}

impl HdContentLayer {
    /// This layer, with `lower` filling in what it leaves unset.
    pub fn or(self, lower: Self) -> Self {
        Self { fonts: self.fonts.or(lower.fonts), cursor: self.cursor.or(lower.cursor), fmv: self.fmv.or(lower.fmv) }
    }
}

impl From<HdContent> for HdContentLayer {
    /// Every option set, for writing the whole set of values to the config file.
    fn from(value: HdContent) -> Self {
        Self { fonts: Some(value.fonts), cursor: Some(value.cursor), fmv: Some(value.fmv) }
    }
}

/// The active options, as the UI and the movies read them. [`publish`] keeps this in step with the settings each frame,
/// for code that has no access to the settings resource.
static ACTIVE: RwLock<HdContent> = RwLock::new(ON);

/// The options in force now.
pub fn active() -> HdContent {
    match ACTIVE.read() {
        Ok(value) => *value,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

/// Copies the settings' options to [`active`]. Runs every frame, so a console or config change takes effect at once.
pub fn publish(settings: Res<Settings>) {
    let _trace = blackbox_trace::scope("Settings: publish HD content");
    let Ok(mut active) = ACTIVE.write() else { return };
    *active = settings.hd_content;
}

/// The `[hd_content]` table of the config file. A value that is not a boolean is reported and left unset.
pub fn read(table: &Table) -> HdContentLayer {
    let Some(section) = table.get("hd_content").and_then(Value::as_table) else {
        return HdContentLayer::default();
    };
    HdContentLayer {
        fonts: config_bool(section, "fonts"),
        cursor: config_bool(section, "cursor"),
        fmv: config_bool(section, "fmv"),
    }
}

fn config_bool(section: &Table, name: &str) -> Option<bool> {
    let value = section.get(name)?;
    let parsed = value.as_bool();
    if parsed.is_none() {
        log::warn!("config [hd_content] {name}: expected true or false");
    }
    parsed
}

/// The environment variables. A value that does not parse is ignored.
pub fn from_env(get: impl Fn(&str) -> Option<String>) -> HdContentLayer {
    HdContentLayer {
        fonts: env_bool(&get, FONTS_ENV),
        cursor: env_bool(&get, CURSOR_ENV),
        fmv: env_bool(&get, FMV_ENV),
    }
}

fn env_bool(get: &impl Fn(&str) -> Option<String>, name: &str) -> Option<bool> {
    parse_bool(&get(name)?).ok()
}

/// Puts the options `changes` sets into the `[hd_content]` table of `table`, keeping the rest of the file. A table that
/// is not a table is left alone rather than overwritten.
pub fn merge_into(table: &mut Table, changes: &HdContentLayer) {
    if *changes == HdContentLayer::default() {
        return;
    }
    let entry = table.entry("hd_content").or_insert_with(|| Value::Table(Table::new()));
    let Some(section) = entry.as_table_mut() else { return };
    for (key, value) in [("fonts", changes.fonts), ("cursor", changes.cursor), ("fmv", changes.fmv)] {
        if let Some(v) = value {
            section.insert(key.to_owned(), Value::Boolean(v));
        }
    }
}

/// The command-line switches of the options. Unset switches leave the config file or the environment in charge.
#[derive(Debug, Clone, Default, Args)]
pub struct HdContentArgs {
    /// HD font glyphs keep the stock texture's layout [env NFSMW_HD_FONTS; config `[hd_content] fonts`; default on].
    #[arg(long, conflicts_with = "no_hd_fonts")]
    pub hd_fonts: bool,
    /// HD font glyphs follow the size of the replacement texture.
    #[arg(long)]
    pub no_hd_fonts: bool,
    /// The pointer's hotspot follows an HD cursor texture [env NFSMW_HD_CURSOR; config `[hd_content] cursor`; default on].
    #[arg(long, conflicts_with = "no_hd_cursor")]
    pub hd_cursor: bool,
    /// The pointer keeps its stock hotspot for any texture size.
    #[arg(long)]
    pub no_hd_cursor: bool,
    /// Film movies keep the stock 1024 x 512 framing [env NFSMW_HD_FMV; config `[hd_content] fmv`; default on].
    #[arg(long, conflicts_with = "no_hd_fmv")]
    pub hd_fmv: bool,
    /// Film movies are framed by their own size.
    #[arg(long)]
    pub no_hd_fmv: bool,
}

impl HdContentArgs {
    pub fn layer(&self) -> HdContentLayer {
        HdContentLayer {
            fonts: switch(self.hd_fonts, self.no_hd_fonts),
            cursor: switch(self.hd_cursor, self.no_hd_cursor),
            fmv: switch(self.hd_fmv, self.no_hd_fmv),
        }
    }
}

fn switch(on: bool, off: bool) -> Option<bool> {
    match (on, off) {
        (true, _) => Some(true),
        (false, true) => Some(false),
        (false, false) => None,
    }
}

/// Whether the console key names one of the options.
pub fn is_key(key: &str) -> bool {
    KEYS.contains(&key)
}

/// The option a console key names, for a switch with no value.
pub fn flag<'a>(settings: &'a mut HdContent, key: &str) -> Option<&'a mut bool> {
    match key {
        "hd_fonts" => Some(&mut settings.fonts),
        "hd_cursor" => Some(&mut settings.cursor),
        "hd_fmv" => Some(&mut settings.fmv),
        _ => None,
    }
}

/// The value of an option as the console shows it.
pub fn get(settings: &HdContent, key: &str) -> String {
    let on = match key {
        "hd_fonts" => settings.fonts,
        "hd_cursor" => settings.cursor,
        _ => settings.fmv,
    };
    let text = match on {
        true => "on",
        false => "off",
    };
    text.to_owned()
}

/// Sets an option from a console value (`on` or `off`).
pub fn set(settings: &mut HdContent, key: &str, value: &str) -> Result<(), String> {
    let Some(option) = flag(settings, key) else { return Err(format!("unknown setting {key}")) };
    *option = parse_bool(value)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_three_options_are_on_by_default() {
        assert_eq!(HdContent::default(), HdContent { fonts: true, cursor: true, fmv: true });
        assert_eq!(HdContent::from_layer(HdContentLayer::default()), HdContent::default());
    }

    #[test]
    fn the_higher_layer_wins_and_gaps_fall_through() {
        let high = HdContentLayer { fonts: Some(false), ..Default::default() };
        let low = HdContentLayer { fonts: Some(true), cursor: Some(false), fmv: None };
        let merged = high.or(low);
        assert_eq!(merged, HdContentLayer { fonts: Some(false), cursor: Some(false), fmv: None });
        assert_eq!(HdContent::from_layer(merged), HdContent { fonts: false, cursor: false, fmv: true });
    }

    #[test]
    fn the_config_table_is_read_per_key() {
        let table: Table = "[hd_content]\nfonts = false\ncursor = \"yes\"\n".parse().unwrap();
        assert_eq!(read(&table), HdContentLayer { fonts: Some(false), cursor: None, fmv: None });
        let none: Table = "vsync = true\n".parse().unwrap();
        assert_eq!(read(&none), HdContentLayer::default());
    }

    #[test]
    fn the_environment_is_read_and_bad_values_are_ignored() {
        let env = |name: &str| match name {
            FONTS_ENV => Some("off".to_owned()),
            CURSOR_ENV => Some("maybe".to_owned()),
            _ => None,
        };
        assert_eq!(from_env(env), HdContentLayer { fonts: Some(false), cursor: None, fmv: None });
    }

    #[test]
    fn merging_keeps_the_other_keys_of_the_file() {
        let mut table: Table = "vsync = true\n[hd_content]\nfmv = true\n".parse().unwrap();
        merge_into(&mut table, &HdContentLayer { fonts: Some(false), cursor: None, fmv: Some(false) });
        let section = table["hd_content"].as_table().unwrap();
        assert_eq!(section["fonts"].as_bool(), Some(false));
        assert_eq!(section["fmv"].as_bool(), Some(false));
        assert!(!section.contains_key("cursor"));
        assert_eq!(table["vsync"].as_bool(), Some(true));
    }

    #[test]
    fn the_console_toggles_and_reads_each_option() {
        let mut s = HdContent::default();
        assert!(is_key("hd_cursor") && !is_key("hd_status"));
        assert_eq!(set(&mut s, "hd_fmv", "off"), Ok(()));
        assert_eq!(get(&s, "hd_fmv"), "off");
        assert!(!s.fmv && s.fonts && s.cursor);
        *flag(&mut s, "hd_fonts").unwrap() = false;
        assert_eq!(get(&s, "hd_fonts"), "off");
        assert!(set(&mut s, "hd_cursor", "sometimes").is_err());
        assert!(set(&mut s, "hd_other", "on").is_err());
    }

    #[test]
    fn the_switches_map_to_layers() {
        let args = HdContentArgs { no_hd_fonts: true, hd_cursor: true, ..Default::default() };
        assert_eq!(args.layer(), HdContentLayer { fonts: Some(false), cursor: Some(true), fmv: None });
    }
}

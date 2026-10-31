//! The key of RIDE 6's pak files, and how the mod's tools hand it to the external reader.
//!
//! RIDE 6 keeps its sound banks (`ride6/Content/WwiseAudio/*.bnk`, `Media/*.wem`) in `.pak` files whose index is AES
//! encrypted. The key is the one **supplied by the user**, who found it in this thread:
//! <https://cs.rin.ru/forum/viewtopic.php?f=10&t=100672>. It was **verified by mounting**: with it the reader lists 88
//! mounted paks (44 without it) and 1096 sound banks, and every bank it extracted is a well formed Wwise bank
//! (`docs/formats/ride6-audio.md`).
//!
//! It lives here, in the RIDE 6 mod, and nowhere in `libs/`. The mod's tools use it **by default**; the environment
//! variable `R6_AES` (or the `pak_key` setting of the mod's `config.toml`) replaces it, for a build of the game that is
//! encrypted with another key. The external reader (the scratch CUE4Parse tool) takes the key from the same variable, so
//! [`apply`] sets it on the command that starts the reader and the user need not export anything.

use std::process::Command;

use anyhow::{Result, bail, ensure};

/// The name of the environment variable the external reader and this mod read the key from.
pub const ENV_NAME: &str = "R6_AES";

/// The default key: 32 bytes as 64 hexadecimal digits, no prefix. Supplied by the user (source in the module doc) and
/// verified by mounting RIDE 6's paks.
pub const DEFAULT_KEY_HEX: &str = "097C9BF915F6DFA3C89893D1AF88FEB994EB0B6BDBD9A84B46533FF8EF0EFA3F";

/// Where the key in use came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A command line option (`--pak-key`).
    Argument,
    Environment,
    Config,
    Default,
}

/// The key to use and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PakKey {
    /// 64 upper-case hexadecimal digits.
    pub hex: String,
    pub source: Source,
}

/// Check a key text (`0x` prefix allowed) and return its 64 upper-case digits.
pub fn normalize(text: &str) -> Result<String> {
    let trimmed = text.trim();
    let digits = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")).unwrap_or(trimmed);
    ensure!(digits.len() == 64, "an AES-256 key is 64 hexadecimal digits, this one has {}", digits.len());
    if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("an AES key is hexadecimal digits only");
    }
    Ok(digits.to_ascii_uppercase())
}

/// Pick the key: the environment value if it is set and not empty, else the `pak_key` setting, else the default.
pub fn resolve_from(environment: Option<&str>, config: Option<&str>) -> Result<PakKey> {
    if let Some(text) = environment.filter(|text| !text.trim().is_empty()) {
        return Ok(PakKey { hex: normalize(text)?, source: Source::Environment });
    }
    if let Some(text) = config.filter(|text| !text.trim().is_empty()) {
        return Ok(PakKey { hex: normalize(text)?, source: Source::Config });
    }
    Ok(PakKey { hex: normalize(DEFAULT_KEY_HEX)?, source: Source::Default })
}

/// [`resolve_from`] with the process environment.
pub fn resolve(config: Option<&str>) -> Result<PakKey> {
    resolve_from(std::env::var(ENV_NAME).ok().as_deref(), config)
}

/// Set the key on the command that starts the external reader (its `R6_AES`).
pub fn apply(command: &mut Command, key: &PakKey) {
    command.env(ENV_NAME, &key.hex);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_a_well_formed_key() {
        let key = resolve_from(None, None).unwrap();
        assert_eq!(key.source, Source::Default);
        assert_eq!(key.hex.len(), 64);
        assert_eq!(key.hex, DEFAULT_KEY_HEX);
    }

    #[test]
    fn the_environment_beats_the_setting_and_the_setting_beats_the_default() {
        let env = "0x".to_owned() + &"ab".repeat(32);
        let cfg = "CD".repeat(32);
        let key = resolve_from(Some(&env), Some(&cfg)).unwrap();
        assert_eq!((key.source, key.hex.as_str()), (Source::Environment, "AB".repeat(32).as_str()));
        let key = resolve_from(Some("  "), Some(&cfg)).unwrap();
        assert_eq!((key.source, key.hex), (Source::Config, cfg));
        assert_eq!(resolve_from(Some(""), Some("")).unwrap().source, Source::Default);
    }

    #[test]
    fn a_bad_key_is_refused_not_ignored() {
        assert!(normalize("1234").is_err());
        assert!(normalize(&"zz".repeat(32)).is_err());
        assert!(resolve_from(Some("nope"), None).is_err());
        assert!(resolve_from(None, Some("0x12")).is_err());
    }

    #[test]
    fn the_command_gets_the_key_in_r6_aes() {
        let key = resolve_from(None, None).unwrap();
        let mut command = Command::new("tool");
        apply(&mut command, &key);
        let value = command.get_envs().find(|(name, _)| *name == ENV_NAME).and_then(|(_, value)| value);
        assert_eq!(value.and_then(|v| v.to_str()), Some(DEFAULT_KEY_HEX));
    }
}

//! Settings from the per-user config file.
//!
//! The file is TOML and is shared with `game_dir` (read by `game-install`). Each key is read on its
//! own, so one bad value does not discard the others.

use std::path::Path;
use std::str::FromStr;

use toml::{Table, Value};

use super::partial::Partial;
use crate::app::pacing::MaxFps;

/// Read the layer from `path`. A missing file is an empty layer; a broken one is reported and ignored.
pub fn read(path: &Path) -> Partial {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text, &path.display().to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Partial::default(),
        Err(e) => {
            log::warn!("cannot read {}: {e}", path.display());
            Partial::default()
        }
    }
}

/// Parse the file's text. `origin` names the file in warnings.
pub fn parse(text: &str, origin: &str) -> Partial {
    let table = match text.parse::<Table>() {
        Ok(t) => t,
        Err(e) => {
            log::warn!("ignoring {origin}: {e}");
            return Partial::default();
        }
    };
    Partial {
        backend: field(&table, origin, "backend", |v| text_of(v)?.parse().map_err(|e| format!("{e}"))),
        vsync: field(&table, origin, "vsync", |v| v.as_bool().ok_or_else(|| "expected true or false".to_owned())),
        max_fps: field(&table, origin, "max_fps", |v| match v {
            Value::Integer(n) => MaxFps::from_str(&n.to_string()),
            other => MaxFps::from_str(text_of(other)?),
        }),
    }
}

/// Read `key` through `convert`; a value of the wrong kind is reported and ignored.
fn field<T>(table: &Table, origin: &str, key: &str, convert: impl Fn(&Value) -> Result<T, String>) -> Option<T> {
    convert(table.get(key)?).map_err(|e| log::warn!("{origin}: ignoring `{key}`: {e}")).ok()
}

fn text_of(v: &Value) -> Result<&str, String> {
    v.as_str().ok_or_else(|| "expected a string".to_owned())
}

#[cfg(test)]
mod tests {
    use blackbox_render::Backend;

    use super::*;

    #[test]
    fn reads_values_and_leaves_game_dir_alone() {
        let p = parse("game_dir = 'D:/NFS'\nbackend = 'vulkan'\nvsync = false\nmax_fps = 144\n", "test");
        assert_eq!(p.backend, Some(Backend::Vulkan));
        assert_eq!(p.vsync, Some(false));
        assert_eq!(p.max_fps, Some(MaxFps::from_str("144").unwrap()));
        assert_eq!(parse("max_fps = 'unlocked'", "test").max_fps, Some(MaxFps::default()));
    }

    #[test]
    fn a_bad_value_does_not_discard_the_others() {
        let p = parse("backend = 'dx11'\nvsync = false\n", "test");
        assert_eq!(p.backend, None);
        assert_eq!(p.vsync, Some(false));
    }

    #[test]
    fn broken_or_missing_files_are_empty() {
        assert_eq!(parse("this is = = not toml", "test"), Partial::default());
        assert_eq!(read(Path::new("does/not/exist.toml")), Partial::default());
    }
}

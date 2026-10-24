//! The spec's registry values (Windows only).

use super::Discovered;
use crate::{GameSpec, RegistryValue};

pub(super) fn lookup(spec: &GameSpec) -> Option<Discovered> {
    use super::Source;
    use std::path::PathBuf;

    spec.registry.iter().find_map(|entry| {
        let (dir, hive) = read(entry)?;
        Some(Discovered {
            path: PathBuf::from(dir),
            source: Source::Registry(format!(r"{hive}\{}\{}", entry.key, entry.value)),
        })
    })
}

/// A string value an installer wrote (the install's language, its CD drive), from `HKLM`, else `HKCU`. `None` off
/// Windows and when neither has it.
pub fn registry_string(entry: &RegistryValue) -> Option<String> {
    read(entry).map(|(text, _)| text)
}

/// The value and the name of the hive it was found in.
#[cfg(windows)]
fn read(entry: &RegistryValue) -> Option<(String, &'static str)> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY};

    for (hive, hive_name) in [(HKEY_LOCAL_MACHINE, "HKLM"), (HKEY_CURRENT_USER, "HKCU")] {
        // 32-bit view: where 32-bit installers write on 64-bit Windows.
        let Ok(key) = RegKey::predef(hive).open_subkey_with_flags(entry.key, KEY_READ | KEY_WOW64_32KEY) else {
            continue;
        };
        if let Ok(text) = key.get_value::<String, _>(entry.value) {
            return Some((text, hive_name));
        }
    }
    None
}

#[cfg(not(windows))]
fn read(_entry: &RegistryValue) -> Option<(String, &'static str)> {
    None
}

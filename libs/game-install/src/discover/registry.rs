//! The spec's registry values (Windows only).

use super::Discovered;
use crate::GameSpec;

#[cfg(windows)]
pub(super) fn lookup(spec: &GameSpec) -> Option<Discovered> {
    use super::Source;
    use std::path::PathBuf;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY};

    for entry in spec.registry {
        for (hive, hive_name) in [(HKEY_LOCAL_MACHINE, "HKLM"), (HKEY_CURRENT_USER, "HKCU")] {
            // 32-bit view: where 32-bit installers write on 64-bit Windows.
            let Ok(key) = RegKey::predef(hive).open_subkey_with_flags(entry.key, KEY_READ | KEY_WOW64_32KEY) else {
                continue;
            };
            if let Ok(dir) = key.get_value::<String, _>(entry.value) {
                return Some(Discovered {
                    path: PathBuf::from(dir),
                    source: Source::Registry(format!(r"{hive_name}\{}\{}", entry.key, entry.value)),
                });
            }
        }
    }
    None
}

#[cfg(not(windows))]
pub(super) fn lookup(_spec: &GameSpec) -> Option<Discovered> {
    None
}

//! The cars' recordings, written once per distinct recording.
//!
//! The 2012 cars share most of their sounds: of the 1,338 engine, shift, turbo, backfire and pop files of 67 cars, 1,070 were
//! byte for byte the same as another car's (123 MB of the 173 MB written). A recording is therefore written once, under
//! [`FOLDER`], named after its content; every manifest that uses it names that path.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use anyhow::{Context, Result};

/// Where the shared recordings are, under the mod's `files/`.
pub const FOLDER: &str = "SOUND/MODENGINE/SHARED";

/// The recordings written so far in one run. Cars are converted on several threads, so the set is shared.
#[derive(Default)]
pub struct SoundStore {
    written: Mutex<HashSet<String>>,
}

impl SoundStore {
    /// Write `bytes` as a file of kind `extension` under `root/files/SOUND/MODENGINE/SHARED/`, unless an equal file is already
    /// there, and return the path the manifests name (under `files/`). Equal bytes give the same path.
    pub fn put(&self, root: &Path, extension: &str, bytes: &[u8]) -> Result<String> {
        let name = name_of(extension, bytes);
        let path = format!("{FOLDER}/{name}");
        let fresh = self.written.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).insert(name);
        if !fresh {
            return Ok(path);
        }
        let file = root.join("files").join(FOLDER);
        std::fs::create_dir_all(&file).with_context(|| format!("creating {}", file.display()))?;
        let file = root.join("files").join(&path);
        std::fs::write(&file, bytes).with_context(|| format!("writing {}", file.display()))?;
        Ok(path)
    }
}

/// The file name of a recording: its 64-bit FNV-1a hash and its length, which together say two recordings are the same.
fn name_of(extension: &str, bytes: &[u8]) -> String {
    let mut hash = 0xCBF2_9CE4_8422_2325_u64;
    for &byte in bytes {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("{hash:016X}{:X}.{extension}", bytes.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nfsmw-2012-sounds-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn equal_recordings_share_one_file_and_different_ones_do_not() {
        let dir = temp("store");
        let store = SoundStore::default();
        let a = store.put(&dir, "WAV", b"one recording").unwrap();
        let b = store.put(&dir, "WAV", b"one recording").unwrap();
        let c = store.put(&dir, "WAV", b"another one").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("SOUND/MODENGINE/SHARED/") && a.ends_with(".WAV"));
        assert_eq!(std::fs::read(dir.join("files").join(&a)).unwrap(), b"one recording");
        let files = std::fs::read_dir(dir.join("files").join(FOLDER)).unwrap().count();
        assert_eq!(files, 2, "two distinct recordings, two files");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_kind_is_part_of_the_name() {
        assert_ne!(name_of("WAV", b"x"), name_of("GIN", b"x"));
        assert_eq!(name_of("WAV", b"x"), name_of("WAV", b"x"));
        assert_ne!(name_of("WAV", b"x"), name_of("WAV", b"xx"));
    }

    #[test]
    fn many_threads_writing_the_same_recording_leave_one_whole_file() {
        let dir = temp("threads");
        let store = SoundStore::default();
        let bytes = vec![7u8; 100_000];
        let paths: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..4).map(|_| scope.spawn(|| store.put(&dir, "WAV", &bytes).unwrap())).collect();
            handles.into_iter().map(|handle| handle.join().unwrap()).collect()
        });
        assert!(paths.windows(2).all(|pair| pair[0] == pair[1]));
        assert_eq!(std::fs::read(dir.join("files").join(&paths[0])).unwrap(), bytes);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

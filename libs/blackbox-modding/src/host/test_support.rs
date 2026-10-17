//! Test helpers for the host: a scratch folder that is removed on drop, and manifest text for a mod.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A folder under the system temp directory, unique to this test run, removed when dropped.
pub struct Scratch {
    path: PathBuf,
}

impl Scratch {
    /// Create a new empty scratch folder. `tag` only makes the name easier to find while debugging.
    pub fn new(tag: &str) -> Self {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("blackbox-modding-{tag}-{}-{nanos}-{serial}", std::process::id()));
        fs::create_dir_all(&path).expect("create scratch folder");
        Scratch { path }
    }

    /// The folder path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Create the folder `name` inside the scratch folder and return its path.
    pub fn folder(&self, name: &str) -> PathBuf {
        let dir = self.path.join(name);
        fs::create_dir_all(&dir).expect("create mod folder");
        dir
    }

    /// Write `contents` to `name` inside the scratch folder.
    pub fn write(&self, name: &str, contents: &[u8]) -> PathBuf {
        let file = self.path.join(name);
        fs::write(&file, contents).expect("write scratch file");
        file
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// The call state of a mod callback on this test thread: the thread is the main thread and a callback is running. Keep the
/// scope alive for as long as the test calls host tables; a test of the refusals leaves it out.
pub fn callback_on_main() -> crate::host::gate::CallScope {
    crate::host::gate::mark_main_thread();
    crate::host::gate::enter()
}

/// The manifest text of a mod that supports every game and needs nothing, with the given id and version. `extra` is
/// appended, so a test can add sections.
pub fn manifest_text(id: &str, version: &str, extra: &str) -> String {
    format!(
        "[mod]\nid = \"{id}\"\nname = \"Test {id}\"\nversion = \"{version}\"\n[api]\nblackbox_modding = \"0.1\"\n\
         [games]\nsupported = [\"*\"]\n{extra}\n"
    )
}

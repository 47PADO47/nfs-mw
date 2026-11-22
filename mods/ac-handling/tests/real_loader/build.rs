//! The scaffolding of the loader test: the captured log, a scratch folder, and building the mod's library through cargo.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard, Once};

use log::{Log, Metadata, Record};

pub const ID: &str = "nfsmw.ac-handling";

pub static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());
static SERIAL: Mutex<()> = Mutex::new(());

struct Capture;

impl Log for Capture {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if let Ok(mut lines) = LINES.lock() {
            lines.push(format!("{}", record.args()));
        }
    }

    fn flush(&self) {}
}

static CAPTURE: Capture = Capture;

/// One test at a time (the log is shared), with the captured log emptied.
pub fn serial() -> MutexGuard<'static, ()> {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = log::set_logger(&CAPTURE);
        log::set_max_level(log::LevelFilter::Trace);
    });
    let guard = SERIAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Ok(mut lines) = LINES.lock() {
        lines.clear();
    }
    guard
}

pub fn logged(needle: &str) -> bool {
    LINES.lock().map(|lines| lines.iter().any(|line| line.contains(needle))).unwrap_or(false)
}

pub struct Scratch {
    pub path: PathBuf,
}

impl Scratch {
    pub fn new(tag: &str) -> Self {
        let nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let path = env::temp_dir().join(format!("ac-handling-loader-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).expect("create scratch folder");
        Scratch { path }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn workspace_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2);
    root.expect("the manifest folder is two levels below the workspace root").to_path_buf()
}

pub fn library_file_name() -> String {
    format!("{}ac_handling{}", env::consts::DLL_PREFIX, env::consts::DLL_SUFFIX)
}

/// Build the mod's library through cargo, once for the whole test binary, and return its path.
fn build_library() -> PathBuf {
    static BUILD: Mutex<Option<PathBuf>> = Mutex::new(None);
    let mut built = BUILD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(path) = built.as_ref() {
        return path.clone();
    }
    let mut command = Command::new(env!("CARGO"));
    command
        .current_dir(workspace_root())
        .args(["build", "-p", "ac-handling", "--target-dir", "target/ac-handling-lib"])
        .args(["--config", "profile.dev.codegen-backend=\"llvm\""])
        .arg("--message-format=json-render-diagnostics")
        .stdout(Stdio::piped());
    if !cfg!(debug_assertions) {
        command.arg("--release");
    }
    let output = command.output().expect("run cargo build for ac-handling");
    assert!(output.status.success(), "cargo build of ac-handling failed");
    let messages = String::from_utf8_lossy(&output.stdout);
    let file = library_file_name();
    let library = messages
        .lines()
        .filter(|line| line.contains("\"compiler-artifact\""))
        .find_map(|line| {
            let names = line.split("\"filenames\":[").nth(1)?.split(']').next()?;
            names
                .split(',')
                .map(|name| name.trim().trim_matches('"').replace("\\\\", "/"))
                .find(|name| name.rsplit('/').next() == Some(file.as_str()))
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| panic!("cargo reported no {file} for ac-handling"));
    assert!(library.is_file(), "built library not found at {}", library.display());
    *built = Some(library.clone());
    library
}

/// Put the mod into `root/nfsmw.ac-handling`, with the config text when given.
pub fn install(root: &Path, config: Option<&str>) {
    let dir = root.join(ID);
    fs::create_dir_all(&dir).expect("mod folder");
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("mod.toml");
    fs::copy(manifest, dir.join("mod.toml")).expect("copy manifest");
    fs::copy(build_library(), dir.join(library_file_name())).expect("copy library");
    if let Some(text) = config {
        fs::write(dir.join("config.toml"), text).expect("config");
    }
}

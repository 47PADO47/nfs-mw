//! End-to-end host test: build the `hello` example mod with cargo, load it with the real loader, and check the
//! handshake, event delivery and `destroy`. Negative fixtures (a future ABI, a missing dependency, a missing library)
//! are written into scratch folders and must be refused with a reason, without stopping the mod that is fine.

#![cfg(feature = "host")]

use std::cell::Cell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::Mutex;
use std::sync::Once;

use blackbox_modding::abi::BbTimeOfDay;
use blackbox_modding::host::{
    BackendResult, HostOffer, InterfaceRegistry, ModRuntime, Policy, RejectReason, ReloadError, RunMode, TimeBackend,
};
use log::{Log, Metadata, Record};

/// Every line the `log` crate receives while the tests run. The host and the mod both log through it.
static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

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

fn install_logger() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = log::set_logger(&CAPTURE);
        log::set_max_level(log::LevelFilter::Trace);
    });
}

fn captured() -> Vec<String> {
    LINES.lock().map(|lines| lines.clone()).unwrap_or_default()
}

/// A folder under the system temp directory, removed when dropped.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let path = env::temp_dir().join(format!("blackbox-modding-it-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).expect("create scratch folder");
        Scratch { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The platform's file name of the hello library, as cargo produces it.
fn library_file_name() -> String {
    format!("{}hello_mod{}", env::consts::DLL_PREFIX, env::consts::DLL_SUFFIX)
}

/// The manifest key for this platform: `windows`, `linux` or `macos`.
fn platform_key() -> &'static str {
    match env::consts::OS {
        "windows" => "windows",
        "linux" => "linux",
        "macos" => "macos",
        other => panic!("the host tests do not cover {other}"),
    }
}

/// Build `hello-mod` with the same profile as this test binary, and return the path of the library it produced. Cargo says
/// where it put the library (`--message-format=json`), so the folder layout of the target directory does not matter.
fn build_hello() -> PathBuf {
    let release = !cfg!(debug_assertions);
    let mut command = Command::new(env!("CARGO"));
    command
        .current_dir(workspace_root())
        .args(["build", "-p", "hello-mod", "--message-format=json-render-diagnostics"])
        .stdout(Stdio::piped());
    if release {
        command.arg("--release");
    }
    let output = command.output().expect("run cargo build -p hello-mod");
    assert!(output.status.success(), "cargo build -p hello-mod failed");

    // The test binary sits in `deps` or, with a separate build folder, in `build/<crate>/<hash>/out`: the library is in the
    // profile folder above whichever it is.
    let exe = env::current_exe().expect("test binary path");
    let library = exe.ancestors().skip(1).map(|dir| dir.join(library_file_name())).find(|path| path.is_file());
    library.unwrap_or_else(|| panic!("built library not found above {}", exe.display()))
  
    let messages = String::from_utf8_lossy(&output.stdout);
    let library = artifact_path(&messages, &library_file_name())
        .unwrap_or_else(|| panic!("cargo reported no {} for hello-mod", library_file_name()));
    assert!(library.is_file(), "built library not found at {}", library.display());
    library
  
}

/// The path of the file named `file` among the `filenames` of cargo's `compiler-artifact` messages. The paths are JSON
/// strings, so a Windows path has its backslashes doubled.
fn artifact_path(messages: &str, file: &str) -> Option<PathBuf> {
    messages.lines().filter(|line| line.contains("\"compiler-artifact\"")).find_map(|line| {
        let names = line.split("\"filenames\":[").nth(1)?.split(']').next()?;
        names.split(',').map(|name| name.trim().trim_matches('"').replace("\\\\", "/")).find_map(|name| {
            let ends_with_file = name.rsplit('/').next() == Some(file);
            ends_with_file.then(|| PathBuf::from(name))
        })
    })
}

/// The manifest of the example, as committed in the repository.
fn example_manifest() -> Vec<u8> {
    let path = workspace_root().join("examples").join("mods").join("hello").join("mod.toml");
    fs::read(&path).expect("read the example mod.toml")
}

/// Put the example mod into `root/example.hello`: its manifest and the built library.
fn install_hello(root: &Path, library: &Path) {
    let dir = root.join("example.hello");
    fs::create_dir_all(&dir).expect("mod folder");
    fs::write(dir.join("mod.toml"), example_manifest()).expect("copy manifest");
    fs::copy(library, dir.join(library_file_name())).expect("copy library");
}

fn manifest(id: &str, api: &str, extra: &str) -> String {
    format!(
        "[mod]\nid = \"{id}\"\nname = \"{id}\"\nversion = \"1.0.0\"\n[api]\nblackbox_modding = \"{api}\"\n\
         [games]\nsupported = [\"*\"]\n{extra}\n"
    )
}

fn write_manifest(root: &Path, folder: &str, text: &str) {
    let dir = root.join(folder);
    fs::create_dir_all(&dir).expect("mod folder");
    fs::write(dir.join("mod.toml"), text).expect("manifest");
}

fn offer() -> HostOffer {
    HostOffer::new("testgame", "test")
}

#[test]
fn hello_loads_receives_events_and_is_destroyed_on_unload() {
    install_logger();
    let library = build_hello();
    let scratch = Scratch::new("hello");
    install_hello(scratch.path(), &library);

    let mut runtime = ModRuntime::start(offer(), &Policy::All, &[scratch.path().to_owned()], RunMode::Normal);

    let loaded = runtime.loaded();
    assert_eq!(loaded.len(), 1, "rejected: {:?}", runtime.rejected());
    assert_eq!(loaded[0].id, "example.hello");
    let sha = loaded[0].sha256.clone().expect("a library was loaded");
    assert_eq!(sha.len(), 64, "SHA-256 in hex");
    assert!(runtime.rejected().is_empty());

    let first = runtime.dispatch(7, 1);
    assert_eq!((first.delivered, first.faults), (1, 0));
    let second = runtime.dispatch(8, 2);
    assert_eq!((second.delivered, second.faults), (1, 0));

    runtime.unload_all();
    let lines = captured();
    let has = |needle: &str| lines.iter().any(|line| line.contains(needle));
    assert!(has("[example.hello] hello: loaded"), "load log missing");
    assert!(has("[example.hello] hello: bb.time absent"), "an absent optional interface must not stop the mod");
    assert!(has("[example.hello] hello: event 7 at frame 1 (count 1)"), "first event missing");
    assert!(has("[example.hello] hello: event 8 at frame 2 (count 2)"), "second event missing");
    assert!(has("[example.hello] hello: unloaded after 2 events"), "destroy did not run");
    assert!(has(&format!("sha256={sha}")), "the startup log does not name the hash");
}

/// A clock the test owns. The mod reads it through the real `bb.time` table, so a value the test sets must come back
/// through the mod's log line: host to mod and back.
struct FakeTime {
    hour: Cell<f32>,
    reads: Cell<u32>,
}

impl TimeBackend for FakeTime {
    fn get(&self, _owner: &str) -> BackendResult<BbTimeOfDay> {
        self.reads.set(self.reads.get() + 1);
        Ok(BbTimeOfDay { hour: self.hour.get(), seconds_per_hour: 60.0, frozen: 0, lights_on: 1, _pad: [0; 2] })
    }

    fn set_hour(&self, _owner: &str, hour: f32) -> BackendResult<()> {
        self.hour.set(hour);
        Ok(())
    }

    fn set_day_length(&self, _owner: &str, _seconds_per_hour: f32) -> BackendResult<()> {
        Ok(())
    }

    fn freeze(&self, _owner: &str, _on: bool) -> BackendResult<()> {
        Ok(())
    }
}

#[test]
fn hello_reads_the_clock_through_the_host_interface() {
    install_logger();
    let library = build_hello();
    let scratch = Scratch::new("clock");
    install_hello(scratch.path(), &library);
    let clock = Rc::new(FakeTime { hour: Cell::new(13.5), reads: Cell::new(0) });
    let mut registry = InterfaceRegistry::new();
    registry.add_time(clock.clone());

    let mut runtime =
        ModRuntime::start_with(offer(), registry, &Policy::All, &[scratch.path().to_owned()], RunMode::Normal);
    assert_eq!(runtime.loaded().len(), 1, "rejected: {:?}", runtime.rejected());
    assert_eq!(clock.reads.get(), 0, "load does not read the clock: no scene exists yet");
    runtime.dispatch(7, 1);
    assert_eq!(clock.reads.get(), 1, "the mod reads the clock once, on its first event");

    runtime.unload_all();
    let line = "[example.hello] hello: time 13.50 h, 60 s per hour, frozen false";
    assert!(captured().iter().any(|logged| logged == line), "the clock value did not come back through the mod");
}

#[test]
fn a_developer_run_loads_a_copy_so_the_original_can_be_replaced_and_reloaded() {
    install_logger();
    let library = build_hello();
    let scratch = Scratch::new("dev");
    install_hello(scratch.path(), &library);
    let cache = scratch.path().join("cache");
    let mode = RunMode::Dev { cache_dir: cache.clone() };

    let mut runtime = ModRuntime::start(offer(), &Policy::All, &[scratch.path().to_owned()], mode);
    assert_eq!(runtime.loaded().len(), 1, "rejected: {:?}", runtime.rejected());

    // The original is not locked by the loader: a rebuild may replace it while the mod runs.
    let original = scratch.path().join("example.hello").join(library_file_name());
    fs::copy(&library, &original).expect("replace the original library while the mod is loaded");

    runtime.reload("example.hello").expect("reload in a developer run");
    let report = runtime.dispatch(3, 9);
    assert_eq!((report.delivered, report.faults), (1, 0));
    let copies = fs::read_dir(cache.join("example.hello")).expect("cache folder").count();
    assert_eq!(copies, 2, "one shadow copy per load");
    assert!(matches!(runtime.reload("nobody.here"), Err(ReloadError::NotLoaded(_))));
    runtime.unload_all();
}

#[test]
fn a_future_abi_is_refused_and_the_rest_still_loads() {
    install_logger();
    let scratch = Scratch::new("future");
    write_manifest(scratch.path(), "future.mod", &manifest("future.mod", "1.0", ""));
    write_manifest(scratch.path(), "plain.mod", &manifest("plain.mod", "0.1", ""));

    let runtime = ModRuntime::start(offer(), &Policy::All, &[scratch.path().to_owned()], RunMode::Normal);

    let loaded: Vec<String> = runtime.loaded().into_iter().map(|mod_| mod_.id).collect();
    assert_eq!(loaded, ["plain.mod"]);
    assert_eq!(runtime.rejected().len(), 1);
    assert_eq!(
        runtime.rejected()[0].reason,
        RejectReason::AbiNotOffered { module: "1.0".to_owned(), host: "0.1".to_owned() }
    );
}

#[test]
fn a_missing_dependency_rejects_the_dependant_only() {
    install_logger();
    let scratch = Scratch::new("missing-dep");
    write_manifest(scratch.path(), "app.mod", &manifest("app.mod", "0.1", "[dependencies]\n\"nobody.base\" = \"*\"\n"));
    write_manifest(scratch.path(), "other.mod", &manifest("other.mod", "0.1", ""));

    let runtime = ModRuntime::start(offer(), &Policy::All, &[scratch.path().to_owned()], RunMode::Normal);

    let loaded: Vec<String> = runtime.loaded().into_iter().map(|mod_| mod_.id).collect();
    assert_eq!(loaded, ["other.mod"]);
    let reason = &runtime.rejected()[0];
    assert_eq!(reason.name(), "app.mod");
    assert_eq!(reason.reason, RejectReason::DependencyMissing { id: "nobody.base".to_owned() });
}

#[test]
fn a_missing_library_file_does_not_stop_a_second_mod() {
    install_logger();
    let library = build_hello();
    let scratch = Scratch::new("missing-lib");
    install_hello(scratch.path(), &library);
    let key = platform_key();
    let text = manifest("broken.mod", "0.1", &format!("[library]\n{key} = \"broken{}\"\n", env::consts::DLL_SUFFIX));
    write_manifest(scratch.path(), "broken.mod", &text);

    let mut runtime = ModRuntime::start(offer(), &Policy::All, &[scratch.path().to_owned()], RunMode::Normal);

    let loaded: Vec<String> = runtime.loaded().into_iter().map(|mod_| mod_.id).collect();
    assert_eq!(loaded, ["example.hello"]);
    assert_eq!(runtime.rejected()[0].name(), "broken.mod");
    assert!(matches!(runtime.rejected()[0].reason, RejectReason::LibraryMissing { .. }));
    assert_eq!(runtime.dispatch(1, 0).delivered, 1);
    runtime.unload_all();
}

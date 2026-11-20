//! End-to-end test of the `bb.handling` host seam: build the `handling-probe` fixture mod with cargo, load it with the real
//! loader against a fake game, and check the registration, a real call through a real vehicle, the quarantine of the
//! `panics` and `nan` variants after five faults, a model that is not handled, a second differently built model, the unload,
//! and the refusal of the hook in an online session. The pure models of the probe have their own tests in its folder.

#![cfg(feature = "host")]

use std::cell::{Cell, RefCell};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Mutex, Once};

use blackbox_modding::abi::{BB_DRIVE_REAR, BB_ERR_DENIED, BB_ERR_NOT_READY, BB_ERR_STALE, BB_ERR_UNSUPPORTED};
use blackbox_modding::host::{
    BackendResult, ConfigBackend, HandlingBackend, HostOffer, InterfaceRegistry, ModRuntime, Policy, RejectReason,
    RunMode, TyreHook, VehicleSpecSheet, VehicleState, VehiclesBackend,
};
use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle, VehicleSpec};
use log::{Log, Metadata, Record};

const ID: &str = "example.handling-probe";

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

fn logged(needle: &str) -> bool {
    LINES.lock().map(|lines| lines.iter().any(|line| line.contains(needle))).unwrap_or(false)
}

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let path = env::temp_dir().join(format!("blackbox-modding-probe-{tag}-{}-{nanos}", std::process::id()));
        fs::create_dir_all(&path).expect("create scratch folder");
        Scratch { path }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn workspace_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2);
    root.expect("the manifest folder is two levels below the workspace root").to_path_buf()
}

fn library_file_name() -> String {
    format!("{}handling_probe{}", env::consts::DLL_PREFIX, env::consts::DLL_SUFFIX)
}

/// Build the probe through its own manifest, once for the whole test binary, and return the library.
fn build_probe() -> PathBuf {
    static BUILD: Mutex<Option<PathBuf>> = Mutex::new(None);
    let mut built = BUILD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(path) = built.as_ref() {
        return path.clone();
    }
    let mut command = Command::new(env!("CARGO"));
    // The repository's dev profile compiles with Cranelift, which cannot catch a panic on Windows: the probe's `panics` variant needs
    // the SDK guard to work, so the fixture is built with LLVM whatever the profile says.
    command
        .current_dir(workspace_root())
        .args(["build", "--manifest-path", "mods/handling-probe/Cargo.toml", "--target-dir", "target/handling-probe"])
        .args(["--config", "profile.dev.codegen-backend=\"llvm\""])
        .arg("--message-format=json-render-diagnostics")
        .stdout(Stdio::piped());
    if !cfg!(debug_assertions) {
        command.arg("--release");
    }
    let output = command.output().expect("run cargo build for mods/handling-probe");
    assert!(output.status.success(), "cargo build of mods/handling-probe failed");
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
        .unwrap_or_else(|| panic!("cargo reported no {file} for handling-probe"));
    assert!(library.is_file(), "built library not found at {}", library.display());
    *built = Some(library.clone());
    library
}

/// Put the probe into `root/example.handling-probe`, with a `config.toml` that picks `model` when given.
fn install_probe(root: &Path, model: Option<&str>) {
    let dir = root.join(ID);
    fs::create_dir_all(&dir).expect("mod folder");
    let manifest = workspace_root().join("mods").join("handling-probe").join("mod.toml");
    fs::copy(manifest, dir.join("mod.toml")).expect("copy manifest");
    fs::copy(build_probe(), dir.join(library_file_name())).expect("copy library");
    if let Some(model) = model {
        fs::write(dir.join("config.toml"), format!("model = \"{model}\"\n")).expect("config");
    }
}

/// The fake game: keeps the hook, can be online, and answers the car's spec.
#[derive(Default)]
struct Game {
    hooks: RefCell<Vec<(u64, TyreHook)>>,
    online: Cell<bool>,
}

impl HandlingBackend for Game {
    fn register_tyre(&self, owner: &str, hook: TyreHook) -> BackendResult<u64> {
        if self.online.get() {
            log::warn!("mod {owner}: bb.handling refused in an online session");
            return Err(BB_ERR_DENIED);
        }
        let handle = self.hooks.borrow().len() as u64 + 1;
        self.hooks.borrow_mut().push((handle, hook));
        Ok(handle)
    }

    fn unregister_tyre(&self, _owner: &str, handle: u64) -> BackendResult<()> {
        let mut hooks = self.hooks.borrow_mut();
        let index = hooks.iter().position(|(known, _)| *known == handle).ok_or(BB_ERR_STALE)?;
        hooks.remove(index).1.retire();
        Ok(())
    }
}

impl VehiclesBackend for Game {
    fn state(&self, _owner: &str) -> BackendResult<VehicleState> {
        Err(BB_ERR_NOT_READY)
    }

    fn teleport(&self, _owner: &str, _position: [f32; 3], _heading: f32) -> BackendResult<()> {
        Err(BB_ERR_NOT_READY)
    }

    fn repair(&self, _owner: &str) -> BackendResult<()> {
        Err(BB_ERR_NOT_READY)
    }

    fn spawn_traffic(&self, _owner: &str, _position: [f32; 3], _heading: f32) -> BackendResult<u64> {
        Err(BB_ERR_UNSUPPORTED)
    }

    fn add_nos(&self, _owner: &str, _amount: f32) -> BackendResult<()> {
        Err(BB_ERR_NOT_READY)
    }

    fn spec(&self, _owner: &str) -> BackendResult<VehicleSpecSheet> {
        let mut gear_ratios = [0.0; 8];
        gear_ratios[..3].copy_from_slice(&[3.6, 2.2, 1.5]);
        Ok(VehicleSpecSheet {
            revision: 1,
            mass_kg: 1500.0,
            gear_count: 3,
            gear_ratios,
            drive_layout: BB_DRIVE_REAR,
            max_power_w: 200_000.0,
            ..VehicleSpecSheet::default()
        })
    }
}

impl ConfigBackend for Game {
    fn main_section(&self, _owner: &str) -> BackendResult<Option<String>> {
        Ok(None)
    }
}

/// A started runtime with the probe loaded in `model`, and the fake game behind it.
struct Rig {
    runtime: ModRuntime,
    game: Rc<Game>,
    _scratch: Scratch,
}

fn rig(tag: &str, model: Option<&str>, online: bool) -> Rig {
    install_logger();
    let scratch = Scratch::new(tag);
    install_probe(&scratch.path, model);
    let game = Rc::new(Game::default());
    game.online.set(online);
    let mut registry = InterfaceRegistry::new();
    registry.add_handling(game.clone()).add_vehicles(game.clone()).add_config(game.clone());
    let runtime = ModRuntime::start_with(
        HostOffer::new("testgame", "test"),
        registry,
        &Policy::All,
        std::slice::from_ref(&scratch.path),
        RunMode::Normal,
    );
    Rig { runtime, game, _scratch: scratch }
}

fn hook_of(rig: &Rig) -> TyreHook {
    rig.game.hooks.borrow().first().expect("the probe registered a hook").1.clone()
}

/// A car resting on flat ground, with `hook` as its tyre model for a vehicle of `role`.
fn car_with(hook: Option<&TyreHook>, role: u32) -> Vehicle {
    let ground = FlatGround::new(0.0);
    let mut car = Vehicle::new(VehicleSpec::example());
    assert!(car.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
    for _ in 0..180 {
        car.step(FIXED_STEP, &InputState::default(), &ground);
    }
    if let Some(hook) = hook {
        car.set_tyre_model(hook.model(Rc::new(Cell::new(1))), role);
    }
    car
}

fn drive(car: &mut Vehicle, seconds: f32) {
    let ground = FlatGround::new(0.0);
    let input = InputState { throttle: 1.0, ..InputState::default() };
    for _ in 0..(seconds / FIXED_STEP) as usize {
        car.step(FIXED_STEP, &input, &ground);
    }
}

fn fingerprint(car: &Vehicle) -> Vec<u32> {
    let p = car.position();
    let mut bits = vec![p.x.to_bits(), p.y.to_bits(), p.z.to_bits(), car.forward_speed().to_bits()];
    bits.extend((0..4).map(|i| car.wheel(i).simulated_angular_velocity.to_bits()));
    bits
}

#[test]
fn the_linear_probe_registers_reads_the_spec_and_drives_a_real_car() {
    let rig = rig("linear", None, false);
    assert_eq!(rig.runtime.loaded().len(), 1, "rejected: {:?}", rig.runtime.rejected());
    assert_eq!(rig.game.hooks.borrow().len(), 1);
    assert!(logged("handling-probe: registered the Linear tyre hook"));
    let hook = hook_of(&rig);
    assert_eq!(hook.owner(), ID);

    let mut car = car_with(Some(&hook), 0);
    drive(&mut car, 5.0);
    assert!(car.forward_speed() > 6.0, "the car drives on the probe's tyres: {}", car.forward_speed());
    assert!((0..4).all(|i| car.wheel(i).simulated_angular_velocity.is_finite()));
    assert_eq!(car.tyre_model_faults(), 0);
    assert_eq!(hook.stats().faults(), 0);
    assert!(hook.stats().calls() >= 4 * 290, "one call per wheel per tick: {}", hook.stats().calls());
    assert!(
        logged("handling-probe: spec revision 1: 1500 kg, 3 gears, 200000 W"),
        "the spec is readable inside the hook"
    );
}

#[test]
fn the_fixture_built_for_one_zero_loads_against_the_one_two_host() {
    let mut registry = InterfaceRegistry::new();
    registry.add_handling(Rc::new(Game::default()));
    let offered =
        registry.offers().into_iter().find(|offer| offer.id == "bb.handling").expect("bb.handling is offered");
    assert_eq!((offered.version.major, offered.version.minor), (1, 2), "the host offers suspension and controls too");
    let rig = rig("one-zero", None, false);
    assert_eq!(
        rig.runtime.loaded().len(),
        1,
        "a mod that asks for an older minor still loads: {:?}",
        rig.runtime.rejected()
    );
    assert_eq!(rig.game.hooks.borrow().len(), 1, "and registers its tyre hook as before");
}

#[test]
fn the_probe_declines_a_vehicle_that_is_not_the_players() {
    let rig = rig("role", None, false);
    let hook = hook_of(&rig);
    let mut plain = car_with(None, 0);
    let mut cop = car_with(Some(&hook), 2);
    drive(&mut plain, 3.0);
    drive(&mut cop, 3.0);
    assert!(hook.stats().calls() > 0, "the hook was asked");
    assert_eq!(fingerprint(&plain), fingerprint(&cop), "a declined car runs the built-in tyres, bit for bit");
    assert_eq!(hook.stats().faults(), 0);
}

#[test]
fn a_second_model_with_a_different_law_drives_through_the_same_hook() {
    let linear = rig("law-linear", Some("linear"), false);
    let circle = rig("law-circle", Some("friction-circle"), false);
    assert!(logged("registered the FrictionCircle tyre hook"));
    let mut a = car_with(Some(&hook_of(&linear)), 0);
    let mut b = car_with(Some(&hook_of(&circle)), 0);
    drive(&mut a, 4.0);
    drive(&mut b, 4.0);
    assert!(a.forward_speed() > 4.0 && b.forward_speed() > 4.0, "{} {}", a.forward_speed(), b.forward_speed());
    assert_ne!(fingerprint(&a), fingerprint(&b), "two models, two cars");
    assert_eq!((a.tyre_model_faults(), b.tyre_model_faults()), (0, 0));
}

#[test]
fn not_handled_leaves_the_built_in_tyres_and_counts_no_fault() {
    let rig = rig("not-handled", Some("not-handled"), false);
    let hook = hook_of(&rig);
    let mut plain = car_with(None, 0);
    let mut declined = car_with(Some(&hook), 0);
    drive(&mut plain, 3.0);
    drive(&mut declined, 3.0);
    assert_eq!(fingerprint(&plain), fingerprint(&declined));
    assert_eq!(hook.stats().faults(), 0);
    assert!(!hook.stats().quarantined());
    assert!(declined.has_tyre_model(), "declining does not retire the model");
}

#[test]
fn a_panicking_probe_is_quarantined_after_five_faults_and_the_car_keeps_driving() {
    let mut rig = rig("panics", Some("panics"), false);
    let hook = hook_of(&rig);
    let mut car = car_with(Some(&hook), 0);
    drive(&mut car, 1.0);
    assert!(logged("mod panicked in tyre hook"), "the SDK reports the panic through the host log");
    assert_eq!(hook.stats().faults(), 5, "five faults, then no more calls");
    assert!(hook.stats().quarantined());
    assert!(!car.has_tyre_model(), "the vehicle dropped the retired model");
    assert!(car.forward_speed() > 3.0 && car.forward_speed().is_finite(), "the built-in tyres took over");
    let calls = hook.stats().calls();
    drive(&mut car, 0.5);
    assert_eq!(hook.stats().calls(), calls, "a quarantined hook is never called again");

    // The runtime counts the same faults against the mod with its own counter, and quarantines it on the fifth.
    for _ in 0..4 {
        assert!(!rig.runtime.record_fault(ID));
    }
    assert!(rig.runtime.record_fault(ID));
    assert!(rig.runtime.loaded()[0].quarantined);
}

#[test]
fn a_nan_probe_is_refused_by_the_vehicle_and_quarantined() {
    let rig = rig("nan", Some("nan"), false);
    let hook = hook_of(&rig);
    let mut plain = car_with(None, 0);
    let mut car = car_with(Some(&hook), 0);
    drive(&mut plain, 1.0);
    drive(&mut car, 1.0);
    assert_eq!(car.tyre_model_faults(), 5, "the vehicle refused five answers");
    assert_eq!(hook.stats().faults(), 5);
    assert!(hook.stats().quarantined());
    assert!(!car.has_tyre_model());
    assert!(car.position().is_finite() && car.forward_speed().is_finite());
    assert_eq!(fingerprint(&plain), fingerprint(&car), "a refused answer leaves the built-in result");
}

#[test]
fn an_online_session_refuses_the_hook_and_the_mod_does_not_load() {
    let rig = rig("online", None, true);
    assert!(rig.runtime.loaded().is_empty());
    assert!(rig.game.hooks.borrow().is_empty());
    assert!(logged("bb.handling refused in an online session"), "the refusal has a one-line reason in the log");
    let rejected = &rig.runtime.rejected()[0];
    assert!(matches!(&rejected.reason, RejectReason::LoadFailed(_)), "{:?}", rejected.reason);
}

#[test]
fn an_unknown_model_name_rejects_the_mod() {
    let rig = rig("unknown", Some("quantum"), false);
    assert!(rig.runtime.loaded().is_empty());
    assert!(matches!(&rig.runtime.rejected()[0].reason, RejectReason::LoadFailed(_)));
}

#[test]
fn unloading_the_mod_removes_the_hook_and_a_car_that_still_holds_it_stops_calling() {
    let mut rig = rig("unload", None, false);
    let hook = hook_of(&rig);
    let mut car = car_with(Some(&hook), 0);
    drive(&mut car, 0.5);
    let calls = hook.stats().calls();
    assert!(calls > 0);
    rig.runtime.unload_all();
    assert!(rig.game.hooks.borrow().is_empty(), "the probe unregistered its hook");
    assert!(logged("handling-probe: unloaded the Linear tyre hook"));
    drive(&mut car, 0.5);
    assert_eq!(hook.stats().calls(), calls, "the library is gone: the retired hook is not called");
    assert!(!car.has_tyre_model());
}

//! Against the banks of a real install. Expected numbers are the ones measured on the PC banks.

use std::path::PathBuf;

use crate::{ClassCall, Host, Instance, ModuleBank, PlayerInputs, SampleEntry, VoiceState, input};

/// Keeps voices alive and records how they start.
#[derive(Default)]
struct Alive {
    live: Vec<usize>,
    starts: Vec<(usize, u32, i32)>,
    objects: Vec<Vec<i32>>,
}

impl Host for Alive {
    fn play(&mut self, player: usize, entry: &SampleEntry, inputs: &PlayerInputs) -> bool {
        self.live.push(player);
        self.starts.push((player, entry.index, inputs.get(input::VOLUME).unwrap_or(-1)));
        true
    }
    fn stop(&mut self, player: usize) {
        self.live.retain(|&p| p != player);
    }
    fn pause(&mut self, _: usize) {}
    fn resume(&mut self, _: usize, _: &PlayerInputs) {}
    fn update(&mut self, player: usize, _: &PlayerInputs) -> VoiceState {
        VoiceState { playing: self.live.contains(&player), elapsed_ms: 0, remaining_ms: 0 }
    }
    fn class_call(&mut self, call: &ClassCall<'_>) -> i32 {
        self.objects.push(call.params.to_vec());
        i32::from(!call.released)
    }
}

fn banks() -> Option<Vec<PathBuf>> {
    let root = PathBuf::from(std::env::var_os("NFSMW_GAME_DIR")?).join("SOUND");
    let mut out = Vec::new();
    for dir in std::fs::read_dir(&root).ok()?.flatten() {
        let Ok(files) = std::fs::read_dir(dir.path()) else { continue };
        out.extend(
            files.flatten().map(|f| f.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("abk"))),
        );
    }
    out.sort();
    Some(out)
}

fn load(name: &str) -> Option<ModuleBank> {
    let path = banks()?.into_iter().find(|p| p.file_name().is_some_and(|n| n.eq_ignore_ascii_case(name)))?;
    ModuleBank::parse(&std::fs::read(path).ok()?).ok()
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn every_shipped_module_runs_without_error() {
    let Some(paths) = banks() else { return };
    assert!(paths.len() >= 300, "{} banks", paths.len());
    let mut modules = 0;
    for path in paths {
        let bank =
            ModuleBank::parse(&std::fs::read(&path).unwrap()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for (i, module) in bank.modules().iter().enumerate() {
            let mut inst = Instance::new(&bank, i, 1).unwrap();
            let mut host = Alive::default();
            // Every parameter at a few different values, to reach the branches of the graph.
            for value in [0, 1, 1000, 20000] {
                inst.set_class_data(&vec![value; inst.class_data_len()]);
                for _ in 0..30 {
                    inst.update(1000.0 / 60.0, &mut host)
                        .unwrap_or_else(|e| panic!("{} module {i}: {e}", path.display()));
                }
            }
            modules += 1;
            assert_eq!(usize::from(module.players), inst.players());
        }
    }
    assert_eq!(modules, 386);
}

/// The engine module of the M3's bank, with the parameters the car sound gives it.
fn engine(rpm: i32, torque: i32) -> (Instance, Alive) {
    let bank = load("CAR_66_ENG_MB_EE.abk").expect("engine bank");
    let mut inst = Instance::new(&bank, 0, 1).unwrap();
    let mut host = Alive::default();
    let mut params = [0; 26];
    (params[0], params[1], params[3], params[12]) = (66, rpm, torque, 1);
    for _ in 0..40 {
        inst.set_class_data(&params);
        inst.update(1000.0 / 60.0, &mut host).unwrap();
    }
    (inst, host)
}

fn layer(inst: &Instance, player: usize) -> (i32, Option<i32>) {
    let view = inst.player_view(player).unwrap();
    (view.select, inst.player_inputs(player).get(input::VOLUME).filter(|_| view.control == 1))
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn the_engine_module_layers_samples_over_the_rpm_range() {
    let (inst, host) = engine(4000, 1000);
    let selects: Vec<i32> = (0..inst.players()).map(|p| layer(&inst, p).0).collect();
    assert_eq!(selects, [8, 0, 1, 2, 3, 6, 5, 4], "one sample group member per player");
    let loud: Vec<usize> = (0..inst.players()).filter(|&p| layer(&inst, p).1.is_some_and(|v| v > 0)).collect();
    assert_eq!(loud, [2, 3, 5, 6], "four layers sound at 4000 rpm under power");
    assert_eq!(host.live.len(), 4, "and those four are the voices the host holds");
    // Volumes (0 to 0x7FFF) with the PC's float-to-integer rounding (`fistp`: to nearest, ties to even).
    for (player, expected) in [(2, 449), (3, 715), (5, 10430), (6, 24720)] {
        let got = layer(&inst, player).1.unwrap();
        assert!((got - expected).abs() <= 1, "player {player}: {got} vs {expected}");
    }
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn the_engine_module_follows_the_rpm() {
    let pitch = |rpm| {
        let (inst, _) = engine(rpm, 1000);
        let p = (0..inst.players()).find(|&p| layer(&inst, p).1.is_some_and(|v| v > 1000)).unwrap();
        inst.player_inputs(p).get(input::PITCH).unwrap()
    };
    let (low, high) = (pitch(3000), pitch(6000));
    assert!(high > low, "{low} -> {high}");
}

#[test]
#[ignore = "needs a game install; set NFSMW_GAME_DIR"]
fn the_sputter_module_pops_when_the_throttle_lifts() {
    let bank = load("SWTN_CAR_66_MB.abk").expect("sweetener bank");
    let module = bank.module_of("CAR_Sputter").expect("sputter module");
    let mut inst = Instance::new(&bank, module, 1).unwrap();
    let mut host = Alive::default();
    let steps = [(6500, 1000, 1, 0); 60]
        .into_iter()
        .chain([(6200, 0, 0, 0); 30])
        .chain([(5500, 0, 0, 0); 60])
        .chain([(4800, 0, 0, 0); 60]);
    for (rpm, torque, accel, shift) in steps {
        inst.set_class_data(&[66, 1234, rpm, 30000, 0, 0, 0, torque, 0, accel, shift]);
        inst.update(1000.0 / 60.0, &mut host).unwrap();
        host.live.clear();
    }
    assert!(!host.starts.is_empty(), "lifting off the throttle starts sputters");
}

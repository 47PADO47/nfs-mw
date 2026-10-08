use crate::{Error, Host, Instance, ModuleBank, PlayerInputs, SampleEntry, VoiceState, input, minimal_bank};

/// Records the voice calls and keeps every voice alive.
#[derive(Default)]
struct Voices {
    log: Vec<String>,
}

impl Host for Voices {
    fn play(&mut self, player: usize, entry: &SampleEntry, inputs: &PlayerInputs) -> bool {
        self.log.push(format!("play {player} #{} vol {}", entry.index, inputs.get(input::VOLUME).unwrap_or(-1)));
        true
    }
    fn stop(&mut self, player: usize) {
        self.log.push(format!("stop {player}"));
    }
    fn pause(&mut self, player: usize) {
        self.log.push(format!("pause {player}"));
    }
    fn resume(&mut self, player: usize, _: &PlayerInputs) {
        self.log.push(format!("resume {player}"));
    }
    fn update(&mut self, player: usize, inputs: &PlayerInputs) -> VoiceState {
        self.log.push(format!(
            "update {player} vol {} changed {}",
            inputs.get(input::VOLUME).unwrap_or(-1),
            inputs.changed(input::VOLUME)
        ));
        VoiceState { playing: true, elapsed_ms: 0, remaining_ms: 0 }
    }
}

fn instance() -> Instance {
    let bank = ModuleBank::parse(&minimal_bank()).unwrap();
    Instance::new(&bank, 0, 1).unwrap()
}

#[test]
fn the_bank_header_and_module_parse() {
    let bank = ModuleBank::parse(&minimal_bank()).unwrap();
    assert_eq!(bank.modules().len(), 1);
    let module = &bank.modules()[0];
    assert_eq!((module.players, module.class_controllers, module.class_data), (1, 0, true));
    assert_eq!(module.class_name.as_deref(), Some("TEST"));
    assert_eq!(bank.module_of("TEST"), Some(0));
    assert_eq!(bank.module_of("OTHER"), None);
}

#[test]
fn bad_files_are_errors_never_panics() {
    let good = minimal_bank();
    assert!(matches!(ModuleBank::parse(b"RIFF"), Err(Error::BadMagic)));
    for cut in 0..good.len() {
        let _ = ModuleBank::parse(&good[..cut]);
    }
    let bank = ModuleBank::parse(&good).unwrap();
    assert!(matches!(Instance::new(&bank, 5, 1), Err(Error::NoSuchModule(5))));
}

#[test]
fn class_data_reaches_the_player() {
    let mut inst = instance();
    assert_eq!((inst.players(), inst.class_data_len()), (1, 3));
    let mut host = Voices::default();
    inst.set_class_data(&[0, 0, 12000]);
    inst.update(16.0, &mut host).unwrap();
    assert!(host.log.is_empty(), "play control 0 starts nothing");
    inst.set_class_data(&[1, 1, 12000]);
    inst.update(16.0, &mut host).unwrap();
    assert_eq!(host.log[0], "play 0 #8 vol 12000", "select 1 is the second sound");
    inst.set_class_data(&[1, 1, 9000]);
    inst.update(16.0, &mut host).unwrap();
    assert_eq!(host.log.last().unwrap(), "update 0 vol 9000 changed true");
    inst.update(16.0, &mut host).unwrap();
    assert_eq!(host.log.last().unwrap(), "update 0 vol 9000 changed false");
    assert_eq!(inst.class_data(2), 9000);
    let inputs = inst.player_inputs(0);
    assert_eq!(
        (inputs.get(input::PITCH), inputs.get(input::VOLUME), inputs.get(input::AZIMUTH)),
        (Some(4096), Some(9000), None)
    );
    assert!((inputs.pitch_ratio() - 1.0).abs() < 1e-6);
    inst.set_class_data(&[0, 1, 9000]);
    inst.update(16.0, &mut host).unwrap();
    assert_eq!(host.log.last().unwrap(), "stop 0");
}

#[test]
fn destroy_stops_voices_and_ends_updates() {
    let mut inst = instance();
    let mut host = Voices::default();
    inst.set_class_data(&[1, 0, 100]);
    inst.update(16.0, &mut host).unwrap();
    let view = inst.player_view(0).unwrap();
    assert_eq!((view.playing, view.control, view.select), (Some(0), 1, 0));
    inst.destroy(&mut host);
    assert_eq!(host.log.last().unwrap(), "stop 0");
    assert!(inst.is_destroyed());
    let calls = host.log.len();
    inst.update(16.0, &mut host).unwrap();
    assert_eq!(host.log.len(), calls, "a destroyed instance does nothing");
}

#[test]
fn equal_seeds_give_equal_runs() {
    let run = |seed| {
        let bank = ModuleBank::parse(&minimal_bank()).unwrap();
        let mut inst = Instance::new(&bank, 0, seed).unwrap();
        let mut host = Voices::default();
        inst.set_class_data(&[1, 0, 5]);
        inst.update(16.0, &mut host).unwrap();
        host.log
    };
    assert_eq!(run(3), run(3));
}

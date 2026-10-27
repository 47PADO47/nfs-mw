//! Checks against the 2012 install (`MW2012_GAME_DIR`), run with `cargo test -p chameleon-sound-graph -- --ignored`.

use chameleon_bundle::Bundle;

use crate::{ItemKind, SoundGraphs, ginsu_resource};

const PORSCHE: &str = "VEHICLES/VEH_1085007_HI.BNDL";

fn bundle_bytes() -> Option<Vec<u8>> {
    let dir = std::env::var_os("MW2012_GAME_DIR")?;
    std::fs::read(std::path::Path::new(&dir).join(PORSCHE)).ok()
}

#[test]
#[ignore = "needs the 2012 install"]
fn the_porsche_engine_sequence_has_three_ginsu_voices_and_an_idle_loop() {
    let Some(bytes) = bundle_bytes() else { return };
    let bundle = Bundle::parse(&bytes).unwrap();
    let graphs = SoundGraphs::read(&bundle);
    let engines: Vec<_> = graphs.driven_by("GetPlayerAudibleRPM").collect();
    assert_eq!(engines.len(), 2, "two Ginsu sets");
    for sequence in engines {
        assert_eq!((sequence.lo, sequence.hi), (1790.0, 7500.0));
        let ginsu = sequence.items.iter().filter(|item| matches!(item.kind, ItemKind::Ginsu(_))).count();
        let waves = sequence.items.iter().filter(|item| matches!(item.kind, ItemKind::Wave(_))).count();
        assert_eq!((ginsu, waves), (3, 1));
    }
    let whine: Vec<_> = graphs.driven_by("GetTransmissionSpeedMph").collect();
    assert_eq!(whine.len(), 1);
    assert_eq!(whine[0].items.len(), 5);
    for item in &whine[0].items {
        let ItemKind::Wave(wave) = &item.kind else { panic!("not a wave") };
        assert!(wave.tracks_driver && wave.pitch.base > 10.0, "{wave:?}");
    }
}

#[test]
#[ignore = "needs the 2012 install"]
fn the_engine_load_sequence_gives_each_ginsu_voice_a_gain_curve() {
    let Some(bytes) = bundle_bytes() else { return };
    let bundle = Bundle::parse(&bytes).unwrap();
    let graphs = SoundGraphs::read(&bundle);
    let engine = graphs.driven_by("GetPlayerAudibleRPM").next().unwrap();
    for item in &engine.items {
        let ItemKind::Ginsu(ginsu) = &item.kind else { continue };
        let found: Vec<_> = graphs.mixer_items("EngineLoad", ginsu.mixer).collect();
        assert_eq!(found.len(), 1, "{:x}", ginsu.mixer);
        assert!(!found[0].1.gains.is_empty() && !found[0].1.gains[0].curve.is_empty());
    }
}

#[test]
#[ignore = "needs the 2012 install"]
fn a_ginsu_resource_converts_and_decodes_with_the_shared_readers() {
    let Some(bytes) = bundle_bytes() else { return };
    let bundle = Bundle::parse(&bytes).unwrap();
    let graphs = SoundGraphs::read(&bundle);
    let engine = graphs.driven_by("GetPlayerAudibleRPM").next().unwrap();
    let ItemKind::Ginsu(ginsu) =
        engine.items.iter().map(|item| &item.kind).find(|k| matches!(k, ItemKind::Ginsu(_))).unwrap()
    else {
        unreachable!()
    };
    let resource = ginsu_resource(&bundle, ginsu.resource).unwrap();
    let gin = resource.to_gin();
    let pcm = ea_audio::gin::decode(&gin).unwrap();
    assert_eq!(pcm.samples.len(), resource.sample_count as usize);
    // Decoded audio of the right alignment is smooth: its first difference carries far less than the signal.
    let signal: f64 = pcm.samples.iter().map(|&s| f64::from(s).powi(2)).sum();
    let diff: f64 = pcm.samples.windows(2).map(|w| (f64::from(w[1]) - f64::from(w[0])).powi(2)).sum();
    println!("samples {} rate {} diff/signal {:.3}", pcm.samples.len(), pcm.sample_rate, diff / signal);
    assert!(diff / signal < 1.0, "the audio does not look decoded: {}", diff / signal);
}

#[test]
#[ignore = "prints a summary"]
fn summary() {
    let Some(bytes) = bundle_bytes() else { return };
    let graphs = SoundGraphs::read(&Bundle::parse(&bytes).unwrap());
    let show = |m: &crate::Modulator| {
        format!(
            "expr={:?} in={:?} out={:?} base={} pts={}",
            m.expression,
            m.input,
            m.output,
            m.base,
            m.curve.points.len()
        )
    };
    for s in graphs.sequences.iter().filter(|s| s.driver.contains("EntityEngineSound") || s.driver.contains("Entities"))
    {
        println!("SEQ {:x} {} [{}..{}]", s.id & 0xffffff, s.driver, s.lo, s.hi);
        for i in &s.items {
            let head = format!("   [{}..{}] in {:?} out {:?}", i.lo, i.hi, i.fade_in, i.fade_out);
            match &i.kind {
                ItemKind::Ginsu(g) => {
                    println!("{head} GINSU {:x} mixer {:x}", g.resource & 0xffffff, g.mixer & 0xffffff)
                }
                ItemKind::Wave(w) => println!(
                    "{head} WAVE {:x?} mixer {:x} track={} gain {} pitch {}",
                    w.waves.iter().map(|x| x & 0xffffff).collect::<Vec<_>>(),
                    w.mixer & 0xffffff,
                    w.tracks_driver,
                    show(&w.gain),
                    show(&w.pitch)
                ),
                ItemKind::Mixer(m) => println!(
                    "{head} MIXER {:x} {:?}",
                    m.channel & 0xffffff,
                    m.gains.iter().map(show).collect::<Vec<_>>()
                ),
                ItemKind::Other(o) => println!("{head} {o}"),
            }
        }
    }
}

#[test]
#[ignore = "needs the 2012 install"]
fn a_converted_ginsu_voice_runs_in_the_ginsu_synthesiser_at_every_frequency_of_its_span() {
    use blackbox_ginsu::{GinsuData, GinsuSynth, GinsuTables, SynthParams};

    let Some(bytes) = bundle_bytes() else { return };
    let bundle = Bundle::parse(&bytes).unwrap();
    let graphs = SoundGraphs::read(&bundle);
    let engine = graphs.driven_by("GetPlayerAudibleRPM").next().unwrap();
    for item in &engine.items {
        let ItemKind::Ginsu(ginsu) = &item.kind else { continue };
        let gin = ginsu_resource(&bundle, ginsu.resource).unwrap().to_gin();
        let (tables, _) = GinsuTables::parse(&gin).unwrap();
        let pcm = ea_audio::gin::decode(&gin).unwrap();
        let data = std::sync::Arc::new(GinsuData::from_i16(tables, &pcm.samples).unwrap());
        let mut synth = GinsuSynth::new(data, engine.lo).unwrap();
        for frequency in [1790.0f32, 3000.0, 5000.0, 7500.0] {
            let mut block = [0.0f32; 4096];
            for _ in 0..8 {
                synth.render(&SynthParams::new(frequency).with_volume(1.0), &mut block);
            }
            let rms = (block.iter().map(|s| s * s).sum::<f32>() / block.len() as f32).sqrt();
            println!("{:x} at {frequency}: rms {rms:.3}", ginsu.resource & 0xffffff);
            assert!(rms > 0.005 && rms.is_finite(), "{frequency}: {rms}");
        }
    }
}

const TESLA: &str = "VEHICLES/VEH_1085764_HI.BNDL";

fn read_car(file: &str) -> Option<SoundGraphs> {
    let dir = std::env::var_os("MW2012_GAME_DIR")?;
    let bytes = std::fs::read(std::path::Path::new(&dir).join(file)).ok()?;
    Some(SoundGraphs::read(&Bundle::parse(&bytes).unwrap()))
}

#[test]
#[ignore = "needs the 2012 install"]
fn the_electric_car_has_two_motor_loops_in_its_player_engine_and_no_engine_speed_sequence() {
    let Some(graphs) = read_car(TESLA) else { return };
    let player = graphs.player_engine().expect("the player's engine sound");
    let speed: Vec<_> = graphs.sequences_in(player).filter(|s| s.driven_by("GetTransmissionSpeedMph")).collect();
    assert_eq!(speed.len(), 1, "the motor sequence");
    let waves = speed[0].items.iter().filter(|item| matches!(item.kind, ItemKind::Wave(_))).count();
    assert_eq!(waves, 2);
    for item in &speed[0].items {
        let ItemKind::Wave(wave) = &item.kind else { continue };
        assert!(!wave.tracks_driver && !wave.pitch.curve.is_empty(), "the pitch is a curve over the window");
        assert!(wave.pitch.expression.is_empty());
    }
    assert!(graphs.sequences_in(player).all(|s| !s.driven_by("GetPlayerAudibleRPM")), "no combustion engine");
    assert!(graphs.sequences_in(player).all(|s| !s.driven_by("GearChangeTimeElapsed")), "no gear change");
    // The opponents' engine sound has its own copy of the motor, which is not the player's.
    let all = graphs.driven_by("GetTransmissionSpeedMph").count();
    assert!(all > speed.len() + 1, "{all} speed-driven sequences in the bundle");
    let whine = graphs.group_of("TransmissionWhine").expect("the gearbox whine object");
    assert_eq!(graphs.sequences_in(whine).filter(|s| s.driven_by("GetTransmissionSpeedMph")).count(), 1);
}

#[test]
#[ignore = "needs the 2012 install"]
fn a_combustion_car_has_no_motor_in_its_player_engine_and_the_same_gearbox_whine() {
    let Some(graphs) = read_car(PORSCHE) else { return };
    let player = graphs.player_engine().expect("the player's engine sound");
    assert!(graphs.sequences_in(player).all(|s| !s.driven_by("GetTransmissionSpeedMph")));
    assert_eq!(graphs.sequences_in(player).filter(|s| s.driven_by("GetPlayerAudibleRPM")).count(), 2);
    let whine = graphs.group_of("TransmissionWhine").expect("the gearbox whine object");
    assert_eq!(graphs.sequences_in(whine).filter(|s| s.items.len() == 5).count(), 1);
}

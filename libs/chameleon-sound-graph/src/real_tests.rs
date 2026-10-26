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
    let engines: Vec<_> = graphs.driven_by("AudibleRPM").collect();
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
    let engine = graphs.driven_by("AudibleRPM").next().unwrap();
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
    let engine = graphs.driven_by("AudibleRPM").next().unwrap();
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

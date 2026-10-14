use super::*;
use crate::audio::{replay::ReplayBackend, shaped::ShapedData, spatial::Params};
use blackbox_fxedit::{Connection, Layout, Node};
use kira::{AudioManager, AudioManagerSettings, Frame, track::MainTrackBuilder};

fn identity() -> Patch {
    let mut layouts = std::array::from_fn(|_| Layout::default());
    layouts[1] = Layout {
        nodes: vec![
            Node { opcode: 0, output_selector: 0, parameters: [0; 3] },
            Node { opcode: 9, output_selector: 0, parameters: [0; 3] },
            Node { opcode: 6, output_selector: 1, parameters: [128, 0, 0] },
            Node { opcode: 6, output_selector: 2, parameters: [64, 0, 0] },
        ],
        connections: [(0, 1), (1, 2), (1, 3)]
            .map(|(source, destination)| Connection { source, destination, source_port: 1, destination_port: 1 })
            .to_vec(),
    };
    Patch { layouts }
}

fn stream(chunks: &[usize], pan: f32, dry: f32) -> Vec<f32> {
    let mut main = MainTrackBuilder::new();
    let bus = main.add_effect(Bus::builder());
    let mut manager = AudioManager::<ReplayBackend>::new(AudioManagerSettings {
        backend_settings: 48000,
        main_track_builder: main,
        ..Default::default()
    })
    .unwrap();
    bus.tx
        .try_send(Prepared { rate: 48000, rows: vec![Some(Processor::new(&identity(), 48000, 2).unwrap())] })
        .unwrap();
    bus.shared.selected.store(0, Ordering::Relaxed);
    let data = kira::sound::static_sound::StaticSoundData {
        sample_rate: 48000,
        frames: Arc::from(vec![Frame::from_mono(0.2); 4096]),
        settings: Default::default(),
        slice: None,
    };
    let params = Params { gain: 1.0, dry, wet: 1.0, pan, mono: true };
    manager.main_track().play(ShapedData { data, wet: bus.sender(Group::Sfx), params }).unwrap();
    let mut samples = Vec::new();
    for &chunk in chunks {
        samples.extend_from_slice(manager.backend_mut().render(chunk));
    }
    samples
}

#[test]
fn wet_only_output_is_independent_of_pan_and_retains_partial_blocks() {
    let chunks = [1, 7, 13, 31, 127, 257, 256, 256, 256, 256];
    let left = stream(&chunks, -1.0, 0.0);
    let right = stream(&chunks, 1.0, 0.0);
    assert_eq!(left, right, "wet send precedes speaker panning");
    assert!(left[..32].iter().all(|sample| sample.abs() < 1e-6), "sixteen-sample FIFO latency");
    assert!((left[128] - 0.1).abs() < 1e-6 && (left[129] - 0.05).abs() < 1e-6);
    let total = chunks.iter().sum();
    let steady = stream(&[total], 0.0, 0.0);
    assert_eq!(left, steady, "backend block grouping never pads or skips graph state");
}

#[test]
#[ignore = "requires installed FXEDIT patches; set NFSMW_GAME_DIR"]
fn installed_wet_bus_runs_every_environment_patch_at_both_output_rates() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("installed wet test requires NFSMW_GAME_DIR");
    for rate in [44100u32, 48000] {
        let dir = game_install::GameDir::open(std::path::PathBuf::from(&path)).unwrap();
        let mut audio = Audio::replay(dir, rate);
        for row in 0..12 {
            audio.update_wet(Some(row));
            audio.render_replay(16);
            assert!(audio.update_wet(Some(row)), "row{row} unsupported at {rate}");
            let data = kira::sound::static_sound::StaticSoundData {
                sample_rate: rate,
                frames: Arc::from(vec![Frame::from_mono(0.1); rate as usize]),
                settings: Default::default(),
                slice: None,
            };
            let params = Params { gain: 1.0, dry: 0.0, wet: 1.0, pan: 1.0, mono: true };
            audio.play_shaped(Group::Engine, data, params).unwrap();
            let mut energy = 0.0;
            for _ in 0..20 {
                for sample in audio.render_replay(rate as usize / 10) {
                    assert!(sample.is_finite());
                    energy += f64::from(*sample).powi(2);
                }
            }
            assert!(energy > 0.001, "row{row} produced no wet output at {rate}");
        }
    }
}

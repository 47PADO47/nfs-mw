//! Checks of the sound of speed against the 2012 install (`MW2012_GAME_DIR`), run with
//! `cargo test -p chameleon-sound-graph -- --ignored`.

use chameleon_bundle::Bundle;

use crate::{ItemKind, SoundGraphs, SpeedSoundSet};

fn install_file(rel: &str) -> Option<Vec<u8>> {
    let dir = std::env::var_os("MW2012_GAME_DIR")?;
    std::fs::read(std::path::Path::new(&dir).join(rel)).ok()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
#[ignore = "needs the 2012 install"]
fn the_speed_sound_set_has_two_wind_layers_of_silence() {
    let Some(bytes) = install_file("SOUND/SPEEDSOUNDS.BNDL") else { return };
    let set = SpeedSoundSet::read(&Bundle::parse(&bytes).unwrap()).unwrap();
    assert_eq!((set.wind.len(), set.world.len(), set.flyby.len()), (2, 4, 2));
    assert!(set.wind[0].values.iter().zip([1.0, 250.0, 0.0, 0.3, 249.0]).all(|(a, b)| near(*a, b)));
    assert!(set.wind[1].values.iter().zip([0.55, 250.0, 0.0, 0.45, 249.0]).all(|(a, b)| near(*a, b)));
    let silent = set.placeholder().expect("every wind and world slot names one sample");
    assert_eq!(silent & 0xff_ffff, 0x79ff);
    assert!(near(set.world[2].values[0], 1.0) && near(set.world[3].values[1], 0.1));
    assert_eq!(set.flyby.iter().map(|layer| layer.samples.len()).collect::<Vec<_>>(), [2, 2]);
    assert!(near(set.flyby[0].value, 37.79) && near(set.flyby[1].value, 5.613));
    let low: Vec<u64> = set.flyby.iter().flat_map(|layer| layer.samples.iter().map(|id| id & 0xff_ffff)).collect();
    assert_eq!(low, [0x7a25, 0x7a26, 0x7a27, 0x7a28]);
    assert!(near(set.values[2], 240.0) && near(set.values[6], 110.0) && near(set.values[10], 150.0));
}

#[test]
#[ignore = "needs the 2012 install"]
fn the_wind_is_a_speed_sequence_of_the_global_sounds() {
    let Some(bytes) = install_file("SOUND/GLOBAL.BNDL") else { return };
    let graphs = SoundGraphs::read(&Bundle::parse(&bytes).unwrap());
    let wind = graphs.wind_sequence().expect("the global sounds have the wind");
    assert_eq!(wind.id & 0xff_ffff, 0x87ea6);
    assert_eq!((wind.lo, wind.hi), (0.0, 180.0));
    let waves: Vec<_> = wind
        .items
        .iter()
        .filter_map(|item| match &item.kind {
            ItemKind::Wave(wave) => Some((item, wave)),
            _ => None,
        })
        .collect();
    assert_eq!(waves.len(), 15);
    // The first three windows, in speed order: 45 to 90, 70 to 138 and 100 to 250, each a pair of loops.
    let windows: Vec<(f32, f32)> = waves.iter().take(6).map(|(item, _)| (item.lo, item.hi)).collect();
    assert_eq!(windows, [(45.0, 90.0), (45.0, 90.0), (70.0, 138.0), (70.0, 138.0), (100.0, 250.0), (100.0, 250.0)]);
    let rates: Vec<f32> = waves.iter().take(6).map(|(_, wave)| wave.pitch.base).collect();
    assert_eq!(rates, [1.0, 1.0, 1.3, 1.3, 2.0, 2.0]);
    let in_air = waves.iter().filter(|(_, wave)| wave.gain.expression.contains("TimeInAir")).count();
    assert_eq!(in_air, 2, "two loops swell while the car is in the air");
    let muted = waves.iter().filter(|(_, wave)| graphs.channel_gains.get(&wave.mixer).is_some_and(|g| *g < 0.001));
    assert_eq!(muted.count(), 2, "two loops sit on channels of gain 0.0001");
}

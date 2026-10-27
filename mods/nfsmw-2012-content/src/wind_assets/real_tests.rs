//! The wind of the real 2012 install. Ignored without `MW2012_GAME_DIR` (the 2012 install).

use super::*;

fn global_sounds() -> Option<Vec<u8>> {
    let root = std::env::var_os("MW2012_GAME_DIR")?;
    std::fs::read(std::path::Path::new(&root).join(GLOBAL_SOUNDS)).ok()
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn the_global_sounds_give_eleven_wind_loops_and_leave_out_the_air_and_the_muted_ones() {
    let Some(bytes) = global_sounds() else { return };
    let bundle = Bundle::parse(&bytes).unwrap();
    let set = build(&SoundGraphs::read(&bundle)).expect("the global sounds have the wind");
    assert_eq!((set.layers.len(), set.in_air, set.muted), (11, 2, 2));
    // Ten distinct loops, one of them (182186) used by two items.
    let mut samples: Vec<u64> = set.layers.iter().map(|layer| layer.sample & 0xff_ffff).collect();
    samples.sort_unstable();
    samples.dedup();
    assert_eq!(samples.len(), 10);
    let rates: Vec<f32> = set.layers.iter().map(|layer| layer.rate).collect();
    assert_eq!(rates[..6], [1.0, 1.0, 1.3, 1.3, 2.0, 2.0]);
    for layer in &set.layers {
        assert!(layer.gain.len() <= 64 && layer.gain.windows(2).all(|pair| pair[0].0 < pair[1].0), "{layer:?}");
        assert!(layer.gain.last().is_some_and(|point| point.0 <= 180.0), "no point beyond the sequence's range");
        assert!((0.0..=1.5).contains(&layer.volume), "{layer:?}");
    }
    // The first loop is silent below its window (45 mph) and at its peak in the middle of it.
    let first = &set.layers[0];
    let level = |speed: f32| first.gain.iter().find(|point| point.0 >= speed).map_or(0.0, |point| point.1);
    assert_eq!(level(40.0), 0.0);
    assert!(level(67.5) > level(50.0) && level(67.5) > 0.1);
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn every_wind_loop_decodes_as_a_loop() {
    let Some(bytes) = global_sounds() else { return };
    let bundle = Bundle::parse(&bytes).unwrap();
    let set = build(&SoundGraphs::read(&bundle)).unwrap();
    let samples = crate::sample::read_all(&bundle);
    assert_eq!(playable(set.layers.clone(), &samples).len(), set.layers.len());
    for layer in &set.layers {
        let sample = samples.iter().find(|sample| sample.id == layer.sample).unwrap();
        assert!((2.8..4.1).contains(&sample.seconds()), "{:x} is {} s", layer.sample, sample.seconds());
    }
}

//! Reads the 2012 install's `POSTFX.BNDL` (`CHAMELEON_GAME_DIR` or `MW2012_GAME_DIR`); ignored without one.

use chameleon_bundle::Bundle;

use super::*;

fn bytes() -> Vec<u8> {
    let root =
        std::env::var_os("CHAMELEON_GAME_DIR").or_else(|| std::env::var_os("MW2012_GAME_DIR")).expect("a game dir");
    std::fs::read(std::path::Path::new(&root).join("POSTFX.BNDL")).expect("POSTFX.BNDL")
}

#[test]
#[ignore = "reads the 2012 install"]
fn the_nitrous_and_overdrive_states_read_with_their_keyframes_and_cubes() {
    let data = bytes();
    let bundle = Bundle::parse(&data).unwrap();
    let nitrous = find_bound(&bundle, "NitrousState.UsingNitrous").expect("the nitrous state");
    assert_eq!((nitrous.state.blend_seconds, nitrous.state.weight), (2.0, 1.0));
    assert!((nitrous.keyframe.vignette.outer[0] - 0.037).abs() < 0.001, "{:?}", nitrous.keyframe);
    assert!((nitrous.keyframe.vignette.b[1] - 0.522).abs() < 0.001);
    assert!((nitrous.keyframe.bloom.colour[2] - 1.0).abs() < 0.001);
    let cube = nitrous.cube.expect("the nitrous cube");
    assert!(!cube.is_identity());
    let grade = cube.grade();
    println!("nitrous grade {grade:?}");
    assert!(grade.tint[0] > 1.05 && grade.tint[2] < 0.95, "{grade:?}");
    let overdrive = find_bound(&bundle, "NitrousState.InOverdrive").expect("the overdrive state");
    assert!(overdrive.cube.expect("the overdrive cube").is_identity(), "the overdrive cube is the identity");
    assert!((overdrive.keyframe.vignette.b[0] - 0.791).abs() < 0.001);
    let heat = find_bound(&bundle, "Status.HeatValue").expect("the heat state");
    assert_eq!(heat.state.blend_seconds, 20.0);
    let cooldown = find_bound(&bundle, "Status.IsInCooldown").expect("the cooldown state");
    assert_eq!(cooldown.state.blend_seconds, 3.0);
}

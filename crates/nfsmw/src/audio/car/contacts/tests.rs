use super::*;

#[test]
fn scrape_creation_gate_uses_quantized_intensity_and_both_descriptions() {
    let sound = |flags: &[&str]| ImpactSound {
        description: flags.iter().map(|flag| (*flag).to_owned()).collect(),
        ..Default::default()
    };
    let wall = sound(&["SMOKABLE", "WALL"]);
    for magnitude in [0.0, 9.0 / 127.0, 9.999 / 127.0] {
        assert!(!admits_scrape(&wall, magnitude));
    }
    for magnitude in [10.0 / 127.0, 1.0] {
        assert!(admits_scrape(&wall, magnitude));
    }
    for flags in [&[][..], &["SMOKABLE"][..], &["WALL"][..], &["SMOKABLE", "CAR"][..]] {
        assert!(admits_scrape(&sound(flags), 0.01));
    }
}

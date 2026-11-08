use super::*;

const COLORS: Colors = Colors {
    background: [10, 16, 28],
    lit: [110, 200, 255],
    dim: [38, 48, 62],
    text: [235, 240, 245],
    label: [140, 150, 160],
    gear: [70, 225, 100],
};

fn count(rgba: &[u8], colour: [u8; 3]) -> usize {
    rgba.as_chunks::<4>().0.iter().filter(|t| t[..3] == colour).count()
}

fn state(speed_kmh: f32, gear: i32, rpm: f32) -> DashState {
    DashState { speed_kmh, gear, rpm, rpm_max: 12_000.0 }
}

#[test]
fn the_bar_lights_as_many_segments_as_the_revs_ask_and_the_rest_stay_dim() {
    let rest = paint(240, 120, &state(0.0, 0, 0.0), &COLORS);
    let half = paint(240, 120, &state(0.0, 0, 6_000.0), &COLORS);
    let full = paint(240, 120, &state(0.0, 0, 12_000.0), &COLORS);
    assert_eq!(count(&rest, COLORS.lit), 0);
    let (rest_dim, half_lit, full_lit) = (count(&rest, COLORS.dim), count(&half, COLORS.lit), count(&full, COLORS.lit));
    assert!(half_lit > 0 && full_lit == 2 * half_lit, "12 of 24 segments against all 24: {half_lit} and {full_lit}");
    assert_eq!(count(&full, COLORS.dim), 0);
    assert_eq!(rest_dim, full_lit, "a segment is lit or dim");
}

#[test]
fn the_speed_and_the_gear_are_drawn_in_their_colours_and_change_with_the_state() {
    let slow = paint(240, 120, &state(7.0, 1, 3_000.0), &COLORS);
    let fast = paint(240, 120, &state(188.0, 5, 9_000.0), &COLORS);
    assert!(count(&slow, COLORS.text) > 0 && count(&slow, COLORS.gear) > 0 && count(&slow, COLORS.label) > 0);
    assert!(count(&fast, COLORS.text) > count(&slow, COLORS.text), "three digits cover more than one");
    assert_ne!(
        paint(240, 120, &state(0.0, 0, 0.0), &COLORS),
        paint(240, 120, &state(0.0, -1, 0.0), &COLORS),
        "N and R differ"
    );
    assert!(fast.as_chunks::<4>().0.iter().all(|t| t[3] == 255), "the picture is opaque");
}

#[test]
fn a_state_has_the_key_of_what_it_paints() {
    assert_eq!(state(50.4, 2, 6_000.0).key(), state(50.2, 2, 6_001.0).key(), "the same whole km/h and segments");
    assert_ne!(state(50.4, 2, 6_000.0).key(), state(51.0, 2, 6_000.0).key());
    assert_ne!(state(50.0, 2, 6_000.0).key(), state(50.0, 3, 6_000.0).key());
    assert_eq!(state(-50.0, -1, 0.0).key().0, 50, "the speed backwards is shown without its sign");
}

#[test]
fn painting_a_display_costs_microseconds() {
    let started = Instant::now();
    let runs = 200;
    for n in 0..runs {
        let state = state(n as f32, 1 + (n % 5), 1_000.0 + 50.0 * n as f32);
        assert_eq!(paint(384, 192, &state, &COLORS).len(), 384 * 192 * 4);
    }
    let each = started.elapsed() / runs as u32;
    println!("a 384 x 192 display: {each:?} to paint");
    assert!(each < Duration::from_millis(5), "painting took {each:?}");
}

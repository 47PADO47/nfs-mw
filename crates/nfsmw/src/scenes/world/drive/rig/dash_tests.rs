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
    assert_eq!(state(50.4, 2, 5_900.0).key(), state(50.2, 2, 5_950.0).key(), "the same whole km/h and segments");
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

fn field(
    channel: nfsmw_data::car::mods::bike_dash::Channel,
    rect: [u32; 4],
) -> nfsmw_data::car::mods::bike_dash::Field {
    use nfsmw_data::car::mods::bike_dash::{Align, Field};
    Field {
        channel,
        rect,
        color: [0, 255, 112],
        background: [20, 30, 40],
        align: Align::Center,
        height: 0,
        segments: 1,
    }
}

#[test]
fn a_field_erases_its_rectangle_and_draws_one_glyph_in_the_clusters_colour() {
    use nfsmw_data::car::mods::bike_dash::Channel;
    let gear = field(Channel::Gear, [0, 0, 40, 60]);
    let neutral = fields::paint_field(&gear, &state(0.0, 0, 0.0));
    let first = fields::paint_field(&gear, &state(30.0, 1, 4_000.0));
    assert_eq!(neutral.len(), 40 * 60 * 4);
    assert!(
        count(&neutral, [0, 255, 112]) > 0 && count(&neutral, [20, 30, 40]) > 0,
        "a glyph on the sampled background"
    );
    assert_eq!(
        count(&neutral, [0, 255, 112]) + count(&neutral, [20, 30, 40]),
        40 * 60,
        "nothing else is left in the rectangle"
    );
    assert_ne!(neutral, first, "N and 1 differ");
    let reverse = fields::paint_field(&gear, &state(-3.0, -1, 2_000.0));
    assert_ne!(reverse, neutral);
}

#[test]
fn a_speed_field_scales_its_digits_to_the_rectangle_and_a_bar_fills_from_the_left() {
    use nfsmw_data::car::mods::bike_dash::Channel;
    let speed = field(Channel::Speed, [0, 0, 120, 40]);
    let (slow, fast) =
        (fields::paint_field(&speed, &state(7.0, 1, 0.0)), fields::paint_field(&speed, &state(143.0, 3, 0.0)));
    assert!(count(&fast, [0, 255, 112]) > count(&slow, [0, 255, 112]));
    let mut bar = field(Channel::RpmBar, [0, 0, 100, 10]);
    let half = fields::paint_field(&bar, &state(0.0, 1, 6_000.0));
    assert_eq!(count(&half, [0, 255, 112]), 50 * 10, "half of a continuous bar");
    assert_eq!(&half[..4], &[0, 255, 112, 255], "from the left");
    bar.segments = 10;
    let segmented = fields::paint_field(&bar, &state(0.0, 1, 6_000.0));
    assert!(
        count(&segmented, [0, 255, 112]) > 0 && count(&segmented, [0, 255, 112]) < 50 * 10,
        "gaps between the lit segments"
    );
}

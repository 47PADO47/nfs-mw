use super::*;

fn look() -> Look {
    Look {
        sprite: "CAR_FLARE0".into(),
        position: [2.0, 0.6, 0.6],
        direction: [1.0, 0.0, 0.0],
        colour: [1.0, 0.5, 0.0],
        alpha: [1.0, 0.5],
        radius: [0.2, 0.6],
        range: [10.0, 110.0],
        falloff: 1.0,
        z_bias: 0.0,
    }
}

fn flare(lit: Lit, side: Side) -> Flare {
    Flare { lit, side, look: look() }
}

#[test]
fn a_sprite_grows_and_dims_in_a_straight_line_between_its_two_distances() {
    let near = appearance(&look(), 1.0, 1.0, 5.0).unwrap();
    let far = appearance(&look(), 1.0, 1.0, 200.0).unwrap();
    let middle = appearance(&look(), 1.0, 1.0, 60.0).unwrap();
    assert_eq!((near.0, far.0), (0.2, 0.6), "the ends hold beyond the range");
    assert!((middle.0 - 0.4).abs() < 1e-5);
    assert_eq!((near.1[3], far.1[3]), (255, 127));
    assert_eq!(near.1[..3], [255, 127, 0], "the colour tints the sprite");
}

#[test]
fn the_angle_off_the_axis_dims_a_sprite_by_the_falloff_and_hides_one_seen_from_behind() {
    let straight = appearance(&look(), 1.0, 1.0, 5.0).unwrap();
    let oblique = appearance(&look(), 1.0, 0.5, 5.0).unwrap();
    assert!(oblique.1[3] < straight.1[3]);
    let omni = Look { falloff: 0.0, ..look() };
    assert_eq!(appearance(&omni, 1.0, 0.1, 5.0).unwrap().1[3], 255, "no falloff: seen from every side");
    assert!(appearance(&look(), 1.0, 0.0, 5.0).is_none());
    assert!(appearance(&look(), 1.0, -0.5, 5.0).is_none());
    assert!(appearance(&look(), 0.0, 1.0, 5.0).is_none(), "an unlit lamp shows nothing");
    assert!(appearance(&look(), 1.0, 1.0, MAX_DISTANCE).is_none());
}

#[test]
fn a_flare_follows_the_state_of_its_lamp() {
    let state = LampState { headlights: true, brake: 0.0, reverse: false, ..Default::default() };
    assert_eq!(flare_level(&flare(Lit::Head, Side::Left), state), 1.0);
    assert_eq!(flare_level(&flare(Lit::Tail, Side::Left), state), 1.0, "the tail lamps burn with the headlights");
    assert_eq!(flare_level(&flare(Lit::Brake, Side::Left), state), 0.0);
    assert_eq!(flare_level(&flare(Lit::Brake, Side::Left), LampState { brake: 0.4, ..state }), 1.0);
    assert_eq!(flare_level(&flare(Lit::Reverse, Side::Left), state), 0.0);
    assert_eq!(flare_level(&flare(Lit::Reverse, Side::Left), LampState { reverse: true, ..state }), 1.0);
    assert_eq!(flare_level(&flare(Lit::Always, Side::Centre), LampState::default()), 1.0);
}

#[test]
fn a_broken_lamp_puts_out_only_its_own_side() {
    let state = LampState { headlights: true, broken: 1, ..Default::default() };
    assert_eq!(flare_level(&flare(Lit::Head, Side::Left), state), 0.0);
    assert_eq!(flare_level(&flare(Lit::Head, Side::Right), state), 1.0);
    let brakes = LampState { brake: 1.0, broken: 16, ..Default::default() };
    assert_eq!(flare_level(&flare(Lit::Brake, Side::Right), brakes), 0.0);
    assert_eq!(flare_level(&flare(Lit::Brake, Side::Left), brakes), 1.0);
    let reverse = LampState { reverse: true, broken: 128, ..Default::default() };
    assert_eq!(flare_level(&flare(Lit::Reverse, Side::Right), reverse), 0.0);
    assert_eq!(
        flare_level(&flare(Lit::Reverse, Side::Centre), reverse),
        1.0,
        "a centre reverse lamp has no damage bit"
    );
}

fn bar(siren: Siren) -> Bar {
    Bar { siren, look: look(), pattern: vec![1.0, 0.0], step_seconds: 0.5, phase_seconds: 0.0 }
}

#[test]
fn a_siren_lamp_flashes_only_while_the_emergency_lights_are_on_and_its_colour_is_not_broken() {
    let on = LampState { emergency: 0x7000, time: 0.1, ..Default::default() };
    assert_eq!(bar_level(&bar(Siren::Red), on), 1.0);
    assert_eq!(bar_level(&bar(Siren::Red), LampState { time: 0.6, ..on }), 0.0, "the pattern's dark step");
    assert_eq!(bar_level(&bar(Siren::Red), LampState { emergency: 0, ..on }), 0.0);
    assert_eq!(bar_level(&bar(Siren::Blue), LampState { broken: emergency::BLUE, ..on }), 0.0);
    assert_eq!(bar_level(&bar(Siren::Red), LampState { broken: emergency::BLUE, ..on }), 1.0);
    assert_eq!(bar_level(&bar(Siren::White), LampState { emergency: emergency::RED, ..on }), 0.0);
}

#[test]
fn a_car_covers_the_kinds_of_lamp_it_gives_sprites_for() {
    let lamps: &'static Lamps = Box::leak(Box::new(Lamps {
        flares: vec![flare(Lit::Head, Side::Left), flare(Lit::Brake, Side::Left)],
        bars: vec![bar(Siren::Red)],
    }));
    let mine = ModLamps { lamps: Some(lamps), textures: Vec::new() };
    assert!(mine.covers(Kind::Head) && mine.covers(Kind::Tail) && mine.covers(Kind::CentreBrake));
    assert!(!mine.covers(Kind::Reverse));
    assert!(mine.covers(Kind::Siren(true)) && mine.covers(Kind::SirenBright(false)) && mine.covers(Kind::SirenWhite));
    assert!(!mine.covers(Kind::Signal(true)));
    let none = ModLamps::default();
    assert!(!none.covers(Kind::Head) && !none.covers(Kind::Siren(true)));
}

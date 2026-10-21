use super::*;

#[test]
fn a_cell_call_asks_the_world_for_the_call_and_leaves_the_screen_alone() {
    let mut screen = NisScreen::default();
    assert_eq!(dispatch(NisEvent::CellCall(13), &mut screen), Some(Request::Call(13)));
    assert_eq!(screen, NisScreen::default());
}

#[test]
fn a_flash_fades_over_its_length_and_then_ends() {
    let mut screen = NisScreen::default();
    assert_eq!(dispatch(NisEvent::ScreenFlash { length: 2.0 }, &mut screen), None);
    assert_eq!(screen.flash_alpha(), Some(1.0));
    screen.advance(1.0);
    assert_eq!(screen.flash_alpha(), Some(0.5));
    screen.advance(1.0);
    assert_eq!(screen.flash_alpha(), None);
}

#[test]
fn a_flash_with_no_length_shows_nothing() {
    let mut screen = NisScreen::default();
    assert_eq!(dispatch(NisEvent::ScreenFlash { length: 0.0 }, &mut screen), None);
    assert_eq!(screen.flash_alpha(), None);
}

#[test]
fn an_overlay_shows_until_the_next_one_replaces_it() {
    let mut screen = NisScreen::default();
    let busted = NisEvent::Overlay { package: "BUSTED_OVERLAY.fng".into(), message: "FLASHTRIGGER".into() };
    assert_eq!(dispatch(busted, &mut screen), None);
    assert_eq!(screen.overlay(), Some(("BUSTED_OVERLAY.fng", "FLASHTRIGGER")));
    screen.advance(60.0);
    assert!(screen.overlay().is_some(), "an overlay has no duration of its own");
    let fade = NisEvent::Overlay { package: "BLACK_FADE_OUT.fng".into(), message: "15_OUT".into() };
    dispatch(fade, &mut screen);
    assert_eq!(screen.overlay(), Some(("BLACK_FADE_OUT.fng", "15_OUT")));
}

#[test]
fn clearing_the_screen_ends_the_flash_and_the_overlay() {
    let mut screen = NisScreen::default();
    dispatch(NisEvent::ScreenFlash { length: 2.0 }, &mut screen);
    dispatch(NisEvent::Overlay { package: "A.fng".into(), message: "B".into() }, &mut screen);
    screen.clear();
    assert_eq!(screen.flash_alpha(), None);
    assert_eq!(screen.overlay(), None);
}

#[test]
fn a_named_effect_asks_the_world_to_play_it_at_its_point() {
    let mut screen = NisScreen::default();
    let event = NisEvent::PlayEffect { name: "exp_lg2".into(), position: [1.0, 2.0, 3.0], magnitude: [0.0, 0.0, 1.0] };
    let request = Request::Effect(EffectRequest {
        name: "exp_lg2".into(),
        position: [1.0, 2.0, 3.0],
        magnitude: [0.0, 0.0, 1.0],
    });
    assert_eq!(dispatch(event, &mut screen), Some(request));
}

#[test]
fn the_events_the_player_only_logs_ask_the_world_for_nothing() {
    let mut screen = NisScreen::default();
    let logged = [
        NisEvent::HideCharacter { name: "Cop".into(), show: false },
        NisEvent::Freeze(true),
        NisEvent::Lights { cop: true, head: false },
        NisEvent::CopLights(true),
        NisEvent::CarShake { shake: 0.003, rate: 0.9, ramp: 1.0, time: 6.0 },
        NisEvent::Other(7),
    ];
    for event in logged {
        assert_eq!(dispatch(event, &mut screen), None);
    }
    assert_eq!(screen, NisScreen::default());
}

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
fn an_overlay_event_is_a_cue_for_its_package_and_draws_nothing_itself() {
    let mut screen = NisScreen::default();
    let flash = || NisEvent::Overlay { package: "BUSTED_OVERLAY.fng".into(), message: "FLASHTRIGGER".into() };
    dispatch(flash(), &mut screen);
    assert!(!screen.is_showing(), "the HUD draws the package; the scene's screen shows no text for it");
    let trigger = fe_hash_upper("FLASHTRIGGER");
    assert_eq!(screen.overlay_cue("busted_overlay.fng"), Some((trigger, 0)));
    dispatch(flash(), &mut screen);
    assert_eq!(screen.overlay_cue("BUSTED_OVERLAY.fng"), Some((trigger, 1)), "a second event is a new cue");
    assert_eq!(screen.overlay_cue("BLACK_FADE_OUT.fng"), None, "a cue is for its own package");
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
        NisEvent::CopLights(true),
        NisEvent::Rain(-1.0),
        NisEvent::PhotoFinish,
        NisEvent::DDaySpeech,
        NisEvent::Inert("ENISTimeOfDay"),
        NisEvent::Other(7),
    ];
    for event in logged {
        assert_eq!(dispatch(event, &mut screen), None);
    }
    assert_eq!(screen, NisScreen::default());
}

#[test]
fn the_black_fade_message_fades_the_screen_to_black_and_the_end_holds_it() {
    let mut screen = NisScreen::default();
    let fade = NisEvent::Overlay { package: "BLACK_FADE_OUT.fng".into(), message: "15_OUT".into() };
    dispatch(fade, &mut screen);
    assert_eq!(screen.black_alpha(), 0.0, "the fade starts clear");
    screen.advance(0.15625);
    assert!((screen.black_alpha() - 0.5).abs() < 1e-4, "half way at half its 300 ticks");
    assert!(screen.is_showing());
    screen.advance(1.0);
    assert_eq!(screen.black_alpha(), 1.0);
    screen.end();
    assert_eq!(screen.black_alpha(), 0.0, "the scene's screen clears at its end");
    assert!(screen.take_held_black(), "a scene that ended black hands the black over");
    assert!(!screen.take_held_black(), "once");
}

#[test]
fn a_scene_that_did_not_reach_black_holds_nothing_and_other_messages_fade_nothing() {
    let mut screen = NisScreen::default();
    dispatch(NisEvent::Overlay { package: "BLACK_FADE_OUT.fng".into(), message: "15_OUT".into() }, &mut screen);
    screen.advance(0.1);
    screen.end();
    assert!(!screen.take_held_black());
    dispatch(NisEvent::Overlay { package: "BUSTED_OVERLAY.fng".into(), message: "15_OUT".into() }, &mut screen);
    assert_eq!(screen.black_alpha(), 0.0, "only the black fade package fades");
    assert_eq!(black_fade("30_IN"), Some((1.0, 0.0, 0.625)));
    assert_eq!(black_fade("FLASHTRIGGER"), None);
}

#[test]
fn the_motion_blur_event_holds_the_blur_off_and_stop_effects_asks_the_world() {
    let mut screen = NisScreen::default();
    assert!(!screen.motion_blur_off());
    assert_eq!(dispatch(NisEvent::MotionBlur(false), &mut screen), None);
    assert!(screen.motion_blur_off());
    dispatch(NisEvent::MotionBlur(true), &mut screen);
    assert!(!screen.motion_blur_off());
    assert_eq!(dispatch(NisEvent::StopEffects, &mut screen), Some(Request::StopEffects));
}

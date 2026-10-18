use super::*;
use nfsmw_career::regions::UnlockState;
use nfsmw_career::regions::unlock::SOUND_ACCEPT;

#[test]
fn make_takes_only_the_region_unlock_screen() {
    let own = screen::SAFEHOUSE_REGION_UNLOCK.to_ascii_lowercase();
    assert!(make(&own, Args::default()).is_some());
    assert!(make("mainmenu_sub.fng", Args { safehouse: true, ..Args::default() }).is_none());
    assert!(make("safehouserivalbio.fng", Args::default()).is_none());
}

#[test]
fn a_frame_converts_to_feng_ticks_at_1200_a_second() {
    assert_eq!(ticks_for(1.0), 1200);
    assert_eq!(ticks_for(0.5), 600);
    assert_eq!(ticks_for(1.0 / 60.0), 20);
    assert_eq!(ticks_for(0.0), 0);
    assert_eq!(ticks_for(-1.0), 0, "a negative time counts as none");
}

#[test]
fn the_leave_closes_the_screen_after_thirty_sixtieths_of_a_second() {
    let mut s = UnlockScreen::new();
    s.input(UnlockInput::Accept);
    let mut closed_on = None;
    for frame in 1..=40 {
        if command_for(s.tick(ticks_for(1.0 / 60.0))).is_some() {
            closed_on = Some(frame);
            break;
        }
    }
    assert_eq!(closed_on, Some(30), "600 ticks is 30 frames of 20 ticks");
}

#[test]
fn only_the_done_event_sends_a_command_and_it_is_resume() {
    assert!(matches!(command_for(UnlockEvent::Done), Some(Command::Resume)));
    assert!(command_for(UnlockEvent::None).is_none());
    assert!(command_for(UnlockEvent::Accepted { sound: SOUND_ACCEPT }).is_none());
}

#[test]
fn accept_and_a_continue_click_both_lead_to_resume() {
    for input in [UnlockInput::Accept, UnlockInput::MouseReleaseOnContinue] {
        let mut s = UnlockScreen::new();
        assert!(matches!(s.input(input), UnlockEvent::Accepted { .. }));
        assert!(matches!(command_for(s.tick(600)), Some(Command::Resume)), "{input:?}");
    }
}

#[test]
fn back_start_and_a_click_elsewhere_never_close_the_screen() {
    let mut s = UnlockScreen::new();
    for input in [UnlockInput::Back, UnlockInput::Start, UnlockInput::MouseReleaseElsewhere] {
        assert!(command_for(s.input(input)).is_none(), "{input:?}");
    }
    assert_eq!(s.state, UnlockState::Waiting);
    assert!(command_for(s.tick(u32::MAX / 2)).is_none(), "nothing times out");
}

#[test]
fn the_continue_hit_box_is_around_its_centre() {
    let centre = Vec2::new(100.0, -200.0);
    assert!(is_continue_point(centre, centre));
    assert!(is_continue_point(centre + Vec2::new(89.0, -23.0), centre));
    assert!(!is_continue_point(centre + Vec2::new(0.0, 40.0), centre));
    assert!(!is_continue_point(centre + Vec2::new(-200.0, 0.0), centre));
}

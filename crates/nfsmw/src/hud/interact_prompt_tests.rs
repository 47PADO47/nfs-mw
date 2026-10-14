use super::interact_prompt::*;

fn prompt(title: &str) -> Prompt {
    Prompt { title: title.to_string(), line: "Press Enter".to_string() }
}

/// Run the fade for `secs` seconds in 1/60 s steps.
fn run(popup: &mut InteractPopup, secs: f32) {
    for _ in 0..(secs * 60.0).round() as usize {
        popup.update(1.0 / 60.0);
    }
}

#[test]
fn a_prompt_fades_in_then_is_up() {
    let mut popup = InteractPopup::default();
    popup.set_target(Some(prompt("Auto shop")));
    popup.update(0.0);
    assert_eq!(popup.drawn().map(|p| p.title.as_str()), Some("Auto shop"), "the card takes the prompt at once");
    assert_eq!(popup.opacity(), 0.0, "and starts from nothing");
    run(&mut popup, FADE_SECS / 2.0);
    assert!(popup.opacity() > 0.0 && popup.opacity() < 1.0, "fading in");
    run(&mut popup, FADE_SECS);
    assert!(popup.is_up());
    assert_eq!(popup.opacity(), 1.0);
    assert_eq!(popup.slide(), 0.0, "slid all the way in");
}

#[test]
fn a_prompt_slides_in_from_the_left_as_it_fades() {
    let mut popup = InteractPopup::default();
    popup.set_target(Some(prompt("Safe house")));
    run(&mut popup, FADE_SECS / 3.0);
    let part = popup.slide();
    assert!(part < 0.0 && part > -SLIDE, "{part}");
    assert!(!popup.is_up(), "not up while a third faded");
}

#[test]
fn leaving_the_zone_fades_the_prompt_out_and_hides_it() {
    let mut popup = InteractPopup::default();
    popup.set_target(Some(prompt("Auto shop")));
    run(&mut popup, 1.0);
    popup.set_target(None);
    run(&mut popup, FADE_SECS / 2.0);
    assert!(popup.drawn().is_some(), "still on the card while it fades");
    assert!(!popup.is_up(), "a prompt that is leaving cannot be acted on");
    run(&mut popup, FADE_SECS);
    assert!(popup.drawn().is_none());
    assert_eq!(popup.opacity(), 0.0);
}

#[test]
fn a_new_zone_replaces_the_card_after_the_old_one_fades() {
    let mut popup = InteractPopup::default();
    popup.set_target(Some(prompt("Auto shop")));
    run(&mut popup, 1.0);
    popup.set_target(Some(prompt("Safe house")));
    run(&mut popup, FADE_SECS / 2.0);
    assert_eq!(popup.drawn().map(|p| p.title.as_str()), Some("Auto shop"));
    run(&mut popup, FADE_SECS * 2.0);
    assert_eq!(popup.drawn().map(|p| p.title.as_str()), Some("Safe house"));
    assert!(popup.is_up());
}

#[test]
fn a_flashed_prompt_holds_the_card_against_the_car() {
    let mut popup = InteractPopup::default();
    popup.set_target(Some(prompt("Auto shop")));
    run(&mut popup, 1.0);
    popup.flash(prompt("Entering auto shop"), 2.0);
    popup.set_target(None);
    run(&mut popup, 1.0);
    assert_eq!(popup.drawn().map(|p| p.title.as_str()), Some("Entering auto shop"));
    run(&mut popup, 1.5);
    popup.set_target(None);
    run(&mut popup, FADE_SECS * 2.0);
    assert!(popup.drawn().is_none(), "after the hold the car's own prompt (none) takes over");
}

#[test]
fn an_empty_card_stays_hidden() {
    let mut popup = InteractPopup::default();
    run(&mut popup, 1.0);
    assert!(popup.drawn().is_none());
    assert!(!popup.is_up());
    assert_eq!(popup.opacity(), 0.0);
    assert_eq!(popup.slide(), -SLIDE);
}

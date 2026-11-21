use super::*;

#[test]
fn an_alert_without_a_follow_up_closes_to_nothing() {
    let a = Alert::new(Text::Literal("Load complete.".into()));
    assert_eq!(a.on_close, None);
}

#[test]
fn an_alert_remembers_what_to_run_when_closed() {
    let a = Alert::then(Text::Literal("Saved.".into()), Command::Resume);
    assert_eq!(a.on_close.as_deref(), Some(&Command::Resume));
}

#[test]
fn game_answers_are_the_original_labels() {
    let c = Confirm::quit_to(QuitTo::Main, false);
    assert_eq!(c.message, Text::Game(QUIT_MESSAGE));
    assert_eq!(c.yes, Text::Game(YES));
    assert_eq!(c.no, Text::Game(NO));
    assert_eq!(*c.on_yes, Command::QuitTo(QuitTo::Main));
}

#[test]
fn the_restart_question_is_the_attention_dialog_with_ok_and_cancel() {
    let c = Confirm::restart_race(Command::RestartRace);
    assert_eq!(c.message, Text::Game(0x4d33_99a8));
    assert_eq!((c.yes, c.no), (Text::Game(0x417b_2601), Text::Game(0x1a29_4dad)));
    assert_eq!(c.title, Some(Text::Game(0x6fd9_1524)), "the attention header");
    assert_eq!(c.look, Some(Look::Attention));
    assert!(!c.focus_yes, "Cancel has the focus");
    assert_eq!(*c.on_yes, Command::RestartRace);
    assert_eq!(c.on_no, None, "Cancel only closes the dialog");
}

#[test]
fn custom_text_stays_as_given() {
    let c = Confirm::new(Text::Literal("Delete car?".into()), Command::QuitGame);
    assert_eq!(c.message, Text::Literal("Delete car?".into()));
    assert_eq!(*c.on_yes, Command::QuitGame);
}

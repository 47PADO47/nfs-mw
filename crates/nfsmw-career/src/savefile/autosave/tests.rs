use super::*;

fn node(id: &str, boss: bool) -> Option<CurrentNode<'_>> {
    Some(CurrentNode { id, boss_race: boss })
}

fn gate(current: Option<CurrentNode<'_>>) -> GateInputs<'_> {
    GateInputs::unforced(current)
}

#[test]
fn the_default_manager_autosaves_in_free_roam() {
    let mut m = AutosaveManager::default();
    assert!(m.can_autosave(&gate(None)));
    assert!(m.can_autosave(&gate(node("14.2.1", false))));
}

#[test]
fn force_wins_over_everything_and_does_not_consume_skip_once() {
    let mut m = AutosaveManager { skip_once: true, option_on: false, ..AutosaveManager::default() };
    let forced = GateInputs {
        force: true,
        context: FrontEndContext(FrontEndContext::ONLINE.0),
        layer_present: false,
        current_node: node("1.8.1", true),
    };
    assert!(m.can_autosave(&forced));
    assert!(m.skip_once, "the force test comes first");
}

#[test]
fn skip_once_drops_exactly_one_evaluation() {
    let mut m = AutosaveManager { skip_once: true, ..AutosaveManager::default() };
    assert!(!m.can_autosave(&gate(None)));
    assert!(!m.skip_once);
    assert!(m.can_autosave(&gate(None)));
}

#[test]
fn each_refusal_of_the_gate() {
    let online =
        GateInputs { context: FrontEndContext(FrontEndContext::CAREER.0 | FrontEndContext::ONLINE.0), ..gate(None) };
    let lan = GateInputs { context: FrontEndContext(FrontEndContext::LAN.0), ..gate(None) };
    let mut m = AutosaveManager::default();
    assert!(!m.can_autosave(&online));
    assert!(!m.can_autosave(&lan));
    assert!(!m.can_autosave(&GateInputs { layer_present: false, ..gate(None) }));
    assert!(!m.can_autosave(&gate(node("1.8.1", false))), "the final event");
    assert!(!m.can_autosave(&gate(node("12.1.1", true))), "a boss race");
    let mut off = AutosaveManager { option_on: false, ..AutosaveManager::default() };
    assert!(!off.can_autosave(&gate(None)), "the option is off");
}

#[test]
fn the_off_by_error_byte_keeps_the_gate_open_with_the_option_off() {
    let mut m = AutosaveManager { option_on: false, off_by_error: true, ..AutosaveManager::default() };
    assert!(m.can_autosave(&gate(None)));
}

#[test]
fn the_gate_order_consumes_skip_once_before_the_context_tests() {
    let mut m = AutosaveManager { skip_once: true, ..AutosaveManager::default() };
    let online = GateInputs { context: FrontEndContext::ONLINE, ..gate(None) };
    assert!(!m.can_autosave(&online));
    assert!(!m.skip_once, "step 2 ran before step 3");
}

#[test]
fn the_profile_loaded_byte_is_not_in_the_gate_but_in_the_run() {
    let mut m = AutosaveManager::default();
    assert!(m.can_autosave(&gate(None)));
    assert_eq!(m.start_run(&gate(None), false, false), RunDecision::Nothing);
    assert!(!m.saving);
    assert_eq!(m.start_run(&gate(None), true, false), RunDecision::Write { overlay: true });
    assert!(m.run && m.saving);
}

#[test]
fn state_b0_does_not_show_the_overlay() {
    let mut m = AutosaveManager::default();
    assert_eq!(m.start_run(&gate(None), true, true), RunDecision::Write { overlay: false });
}

#[test]
fn a_run_with_off_by_error_asks_instead_of_writing() {
    let mut m = AutosaveManager { off_by_error: true, ..AutosaveManager::default() };
    assert_eq!(m.start_run(&gate(None), true, false), RunDecision::AskContinue { overlay: true });
    assert!(!m.saving && !m.run);
}

#[test]
fn the_triggers_are_roaming_start_event_start_and_pursuit_evaded() {
    assert!(Trigger::RoamingStart.posts_event());
    let start = |node, race_flag| Trigger::EventStart { node, race_flag };
    assert!(start(Some("14.2.1"), true).posts_event());
    assert!(!start(Some("14.2.1"), false).posts_event(), "a node without the flag");
    assert!(!start(Some("16.1.0"), true).posts_event(), "the opening race");
    assert!(!start(None, true).posts_event(), "no node");
    assert!(start(Some("1.8.1"), false).posts_event(), "the final event posts whatever the flag");
    assert!(Trigger::PursuitEvaded { event_underway: false }.posts_event());
    assert!(!Trigger::PursuitEvaded { event_underway: true }.posts_event());
}

#[test]
fn a_boss_race_start_posts_the_event_but_the_gate_refuses_it() {
    let trigger = Trigger::EventStart { node: Some("12.8.1"), race_flag: true };
    assert!(trigger.posts_event());
    let mut m = AutosaveManager::default();
    m.event_disposed(&gate(node("12.8.1", true)));
    assert!(!m.pending, "no save happens before a boss race");
}

#[test]
fn a_disposed_event_sets_pending_only_when_the_gate_passes() {
    let mut m = AutosaveManager::default();
    m.event_disposed(&gate(None));
    assert!(m.pending);
    let mut off = AutosaveManager { option_on: false, ..AutosaveManager::default() };
    off.event_disposed(&gate(None));
    assert!(!off.pending);
}

#[test]
fn pending_waits_for_the_world_and_for_an_idle_gameplay_manager() {
    let mut m = AutosaveManager::default();
    m.event_disposed(&gate(None));
    assert_eq!(m.tick(true, &gate(None), true), None, "not armed: the front end is up");
    m.hud_update();
    assert!(m.armed);
    assert_eq!(m.tick(false, &gate(None), true), None, "a notification is waiting");
    assert!(m.pending && m.armed, "both bits stay");
    assert_eq!(m.tick(true, &gate(None), true), Some(RunDecision::Write { overlay: true }));
    assert!(!m.pending && !m.armed);
    assert_eq!(m.tick(true, &gate(None), true), None);
}

#[test]
fn hud_update_arms_only_a_pending_request() {
    let mut m = AutosaveManager::default();
    m.hud_update();
    assert!(!m.armed);
}

#[test]
fn the_first_attempt_after_a_resume_is_dropped() {
    let mut m = AutosaveManager::default();
    assert!(m.resume_sets_skip(16, Some("16.2.2"), false));
    m.event_disposed(&gate(None));
    assert!(!m.pending, "the disposal consumed the skip");
    m.event_disposed(&gate(None));
    assert!(m.pending);
}

#[test]
fn the_resume_sets_skip_in_two_cases_only() {
    let mut m = AutosaveManager::default();
    assert!(!m.resume_sets_skip(16, Some("16.1.0"), false));
    assert!(!m.resume_sets_skip(14, Some("14.1.1"), false));
    assert!(!m.resume_sets_skip(16, None, false));
    assert!(m.resume_sets_skip(14, Some("1.8.1"), true));
    assert!(m.skip_once);
}

#[test]
fn a_failed_autosave_turns_the_option_off_and_asks() {
    let mut m = AutosaveManager::default();
    m.start_run(&gate(None), true, false);
    assert_eq!(m.write_failed(), ContinuePrompt::Cardless);
    assert!(!m.option_on && !m.saving && !m.run);
    let mut again = AutosaveManager { off_by_error: true, ..AutosaveManager::default() };
    assert_eq!(again.write_failed(), ContinuePrompt::NoCard);
}

#[test]
fn yes_re_enables_and_retries_forced_no_closes_with_the_option_off() {
    let mut m = AutosaveManager::default();
    m.write_failed();
    assert_eq!(m.answer_continue(true), ContinueOutcome::RetryForced);
    assert!(m.option_on && m.re_enable);
    m.write_failed();
    assert_eq!(m.answer_continue(false), ContinueOutcome::Closed);
    assert!(!m.option_on);
    assert!(!m.can_autosave(&gate(None)), "the gate stays closed until the Options turn it on");
}

#[test]
fn a_completed_write_clears_the_error_byte() {
    let mut m = AutosaveManager { off_by_error: true, saving: true, run: true, ..AutosaveManager::default() };
    m.write_completed();
    assert!(!m.off_by_error && !m.saving && !m.run);
}

#[test]
fn arming_quiet_mode_stores_the_option_on_and_clears_the_error() {
    let mut m = AutosaveManager { option_on: false, off_by_error: true, ..AutosaveManager::default() };
    m.arm_quiet_mode();
    assert!(m.option_on && !m.off_by_error);
}

#[test]
fn screen_autosave_callers_open_the_screen_under_their_conditions() {
    let mut m = AutosaveManager::default();
    let g = gate(None);
    assert!(ScreenAutosave::Stepper.opens_screen(&mut m, &g));
    assert!(ScreenAutosave::TitleExit { unsaved_changes: true }.opens_screen(&mut m, &g));
    assert!(!ScreenAutosave::TitleExit { unsaved_changes: false }.opens_screen(&mut m, &g));
    assert!(ScreenAutosave::ShopExit { stable_changed: true }.opens_screen(&mut m, &g));
    assert!(!ScreenAutosave::ShopExit { stable_changed: false }.opens_screen(&mut m, &g));
    let no_layer = GateInputs { layer_present: false, ..g };
    assert!(!ScreenAutosave::TitleExit { unsaved_changes: true }.opens_screen(&mut m, &no_layer));
}

#[test]
fn the_stepper_caller_consumes_the_skip_once_byte() {
    let mut m = AutosaveManager { skip_once: true, ..AutosaveManager::default() };
    assert!(!ScreenAutosave::Stepper.opens_screen(&mut m, &gate(None)));
    assert!(!m.skip_once);
}

#[test]
fn the_state_b0_step_picks_create_forced_save_or_the_manual_question() {
    let mut m = AutosaveManager::default();
    assert_eq!(screen_state_b0(&mut m, &gate(None), false), ScreenStep::Create);
    assert_eq!(screen_state_b0(&mut m, &gate(None), true), ScreenStep::ForcedSave);
    let mut off = AutosaveManager { option_on: false, ..AutosaveManager::default() };
    assert_eq!(screen_state_b0(&mut off, &gate(None), true), ScreenStep::ManualQuestion);
}

use super::{FairBustsSettings, ResetPenalty, parse};

const NORMAL_REACH: f32 = 15.0;

#[test]
fn defaults_keep_the_stock_penalty_until_the_enhancement_is_enabled() {
    let d = FairBustsSettings::default();
    assert!(!d.enabled);
    assert!(!d.reset_auto_bust);
    assert_eq!(d.reset_bust_multiplier, 4.0);
    assert_eq!(d.penalty(NORMAL_REACH), ResetPenalty::STOCK, "an absent opt-in keeps the stock reset penalty");
}

#[test]
fn a_missing_or_empty_file_is_the_default() {
    assert_eq!(parse(""), FairBustsSettings::default());
    assert_eq!(parse("[Other]\nEnabled = 0\n"), FairBustsSettings::default(), "only [General] counts");
}

#[test]
fn disabled_is_the_stock_penalty_whatever_else_is_set() {
    let off = parse("[General]\nEnabled = 0\nResetAutoBust = 0\nResetBustMultiplier = 1.0\n");
    assert_eq!(off.penalty(NORMAL_REACH), ResetPenalty::STOCK);
    assert_eq!(ResetPenalty::STOCK, ResetPenalty { reach: 90.0, speed_gate: false, multiplier: 4.0 });
}

#[test]
fn reset_auto_bust_keeps_the_stock_reach_and_skips_the_gate() {
    let stock = parse("[General]\nEnabled = 1\nResetAutoBust = 1\nResetBustMultiplier = 2.5\n");
    assert_eq!(
        stock.penalty(NORMAL_REACH),
        ResetPenalty { reach: 90.0, speed_gate: false, multiplier: 2.5 },
        "the multiplier still applies: the mod patches it regardless"
    );
}

#[test]
fn the_normal_reach_is_the_one_the_caller_passes() {
    let fair = parse("[General]\nEnabled = 1\n");
    assert_eq!(fair.penalty(20.0).reach, 20.0);
    assert!(fair.penalty(20.0).speed_gate);
}

#[test]
fn the_multiplier_clamps_to_zero_to_sixty_four() {
    assert_eq!(parse("[General]\nResetBustMultiplier = 100\n").reset_bust_multiplier, 64.0);
    assert_eq!(parse("[General]\nResetBustMultiplier = -3\n").reset_bust_multiplier, 0.0);
}

#[test]
fn a_malformed_number_or_switch_is_the_default() {
    let got = parse("[General]\nResetBustMultiplier = lots\nEnabled = maybe\nResetAutoBust = yes\n");
    assert_eq!(got, FairBustsSettings::default());
}

#[test]
fn switches_read_zero_one_true_and_false() {
    let got = parse("[General]\nEnabled = false\nResetAutoBust = TRUE\n");
    assert!(!got.enabled);
    assert!(got.reset_auto_bust);
}

#[test]
fn a_later_line_overrides_an_earlier_one() {
    let got = parse("[General]\nResetBustMultiplier = 2\nResetBustMultiplier = 6\n");
    assert_eq!(got.reset_bust_multiplier, 6.0);
}

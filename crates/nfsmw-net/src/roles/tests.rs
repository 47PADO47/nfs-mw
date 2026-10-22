use super::*;

const ALL: [Command; 10] = [
    Command::ChangeSettings,
    Command::Start,
    Command::Cancel,
    Command::Hold,
    Command::Kick,
    Command::Ban,
    Command::Mute,
    Command::Restart,
    Command::ChangeMode,
    Command::SetAdmin,
];

#[test]
fn a_player_may_run_no_command() {
    assert!(ALL.iter().all(|command| !permits(Rank::Player, *command)));
}

#[test]
fn an_admin_may_do_everything_but_name_admins() {
    for command in ALL {
        assert_eq!(permits(Rank::Admin, command), command != Command::SetAdmin, "{command:?}");
    }
    assert!(ALL.iter().all(|command| permits(Rank::Host, *command)));
}

#[test]
fn moderation_only_reaches_a_lower_rank() {
    assert!(may_target(Rank::Host, Rank::Admin));
    assert!(may_target(Rank::Admin, Rank::Player));
    assert!(!may_target(Rank::Admin, Rank::Admin));
    assert!(!may_target(Rank::Admin, Rank::Host));
    assert!(!may_target(Rank::Player, Rank::Player));
}

#[test]
fn a_timed_ban_ends_and_a_session_ban_does_not() {
    let mut bans = Bans::new();
    bans.ban([1; 16], 2, 1_000);
    bans.ban([2; 16], 0, 1_000);
    assert!(bans.is_banned([1; 16], 120_999));
    assert!(!bans.is_banned([1; 16], 121_000));
    assert!(bans.is_banned([2; 16], u64::MAX));
    bans.expire(121_000);
    assert_eq!(bans.len(), 1);
    assert!(bans.lift([2; 16]));
    assert!(!bans.lift([2; 16]));
    assert!(bans.is_empty());
}

#[test]
fn bans_are_bounded_and_forget_the_oldest() {
    let mut bans = Bans::new();
    for index in 0..=MAX_BANS {
        let mut id = [0; 16];
        id[..8].copy_from_slice(&(index as u64).to_le_bytes());
        bans.ban(id, 0, 0);
    }
    assert_eq!(bans.len(), MAX_BANS);
    assert!(!bans.is_banned([0; 16], 0));
}

#[test]
fn a_rebanned_player_has_one_entry() {
    let mut bans = Bans::new();
    bans.ban([3; 16], 1, 0);
    bans.ban([3; 16], 0, 0);
    assert_eq!(bans.len(), 1);
    assert!(bans.is_banned([3; 16], 10_000_000));
}

#[test]
fn local_mutes_hide_a_player_and_are_bounded() {
    let mut mutes = LocalMutes::new();
    assert!(mutes.set([1; 16], true));
    assert!(!mutes.shows([1; 16]));
    assert!(mutes.shows([2; 16]));
    assert!(mutes.set([1; 16], false));
    assert!(mutes.shows([1; 16]));
    for index in 0..MAX_LOCAL_MUTES {
        assert!(mutes.set([index as u8 + 10; 16], true));
    }
    assert!(!mutes.set([255; 16], true));
}

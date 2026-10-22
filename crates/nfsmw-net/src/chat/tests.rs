use super::*;
use crate::message::{ChatText, LeaveReason, NoticeCode, VoteKind};

fn line(channel: ChatChannel, to_slot: Option<u8>, text: &str) -> ChatSend {
    ChatSend { channel, to_slot, text: ChatText::new(text).unwrap() }
}

fn player(slot: u8) -> Sender {
    Sender { slot, team: 1, muted: false }
}

fn members(slot: u8) -> bool {
    slot < 4
}

#[test]
fn a_line_is_relayed_to_its_audience() {
    let mut gate = ChatGate::new();
    let relay = gate.send(player(0), &line(ChatChannel::All, None, "hi"), members, 77, 0).unwrap();
    assert_eq!(relay.audience, Audience::All);
    assert_eq!((relay.message.from_slot, relay.message.tick, relay.message.text.as_str()), (0, 77, "hi"));
    let team = gate.send(player(0), &line(ChatChannel::Team, None, "go"), members, 0, 0).unwrap();
    assert_eq!(team.audience, Audience::Team(1));
    let whisper = gate.send(player(0), &line(ChatChannel::Whisper, Some(2), "psst"), members, 0, 0).unwrap();
    assert_eq!(whisper.audience, Audience::Whisper { from: 0, to: 2 });
}

#[test]
fn the_rate_is_one_a_second_with_a_burst_of_three() {
    let mut gate = ChatGate::new();
    let send = line(ChatChannel::All, None, "spam");
    for _ in 0..3 {
        assert!(gate.send(player(1), &send, members, 0, 5_000).is_ok());
    }
    assert_eq!(gate.send(player(1), &send, members, 0, 5_000), Err(ChatRefusal::RateLimited));
    assert_eq!(gate.send(player(1), &send, members, 0, 5_999), Err(ChatRefusal::RateLimited));
    assert!(gate.send(player(1), &send, members, 0, 6_000).is_ok());
    assert!(gate.send(player(2), &send, members, 0, 6_000).is_ok(), "buckets are per slot");
}

#[test]
fn a_flood_of_a_thousand_lines_passes_only_the_refill() {
    let mut gate = ChatGate::new();
    let send = line(ChatChannel::All, None, "flood");
    let passed = (0..1_000u64).filter(|index| gate.send(player(1), &send, members, 0, index * 10).is_ok()).count();
    assert_eq!(passed, 3 + 9, "burst plus one per second over 9.99 s");
}

#[test]
fn a_muted_sender_spends_no_token() {
    let mut gate = ChatGate::new();
    let muted = Sender { muted: true, ..player(1) };
    let send = line(ChatChannel::All, None, "let me talk");
    for _ in 0..10 {
        assert_eq!(gate.send(muted, &send, members, 0, 0), Err(ChatRefusal::Muted));
    }
    assert!(gate.send(player(1), &send, members, 0, 0).is_ok());
}

#[test]
fn a_whisper_needs_another_member() {
    let mut gate = ChatGate::new();
    let to_self = line(ChatChannel::Whisper, Some(0), "me");
    assert_eq!(gate.send(player(0), &to_self, members, 0, 0), Err(ChatRefusal::UnknownTarget));
    let to_nobody = line(ChatChannel::Whisper, Some(20), "you?");
    assert_eq!(gate.send(player(0), &to_nobody, members, 0, 0), Err(ChatRefusal::UnknownTarget));
}

#[test]
fn quick_messages_check_the_id_and_share_the_bucket() {
    let mut gate = ChatGate::new();
    assert_eq!(gate.quick(player(0), &QuickMessage { id: QUICK_LINES }, 0), Err(ChatRefusal::UnknownQuick));
    assert_eq!(gate.quick(player(0), &QuickMessage { id: 3 }, 0), Ok(QuickRelay { from: 0, id: 3 }));
    assert!(gate.quick(player(0), &QuickMessage { id: EMOTE_BASE + 1 }, 0).is_ok());
    assert!(gate.send(player(0), &line(ChatChannel::All, None, "x"), members, 0, 0).is_ok());
    assert_eq!(gate.quick(player(0), &QuickMessage { id: 3 }, 0), Err(ChatRefusal::RateLimited));
    gate.forget(0);
    assert!(gate.quick(player(0), &QuickMessage { id: 3 }, 0).is_ok());
}

#[test]
fn normalise_strips_invisible_and_reordering_characters() {
    assert_eq!(normalise("  hello\t\n world  ").unwrap().as_str(), "hello world");
    assert_eq!(normalise("a\u{200B}b\u{FEFF}c\u{00AD}d").unwrap().as_str(), "abcd");
    assert_eq!(normalise("evil\u{202E}txt.exe").unwrap().as_str(), "eviltxt.exe");
    assert_eq!(normalise("\u{200D}\u{200D}x\u{200D}\u{200D}y").unwrap().as_str(), "x\u{200D}y");
    assert_eq!(normalise("x\u{FDD0}\u{FFFF}\u{1FFFE}").unwrap().as_str(), "x");
    assert!(normalise("\u{200B}\u{202E}\u{0301} \t").is_none());
}

#[test]
fn normalise_caps_combining_floods_and_length() {
    let zalgo = format!("a{}b", "\u{0301}".repeat(40));
    assert_eq!(normalise(&zalgo).unwrap().as_str(), "a\u{0301}\u{0301}b");
    let accents = "cafe\u{0301} na\u{0308}ive";
    assert_eq!(normalise(accents).unwrap().as_str(), accents);
    let long = "w".repeat(500);
    assert_eq!(normalise(&long).unwrap().chars().count(), MAX_CHAT_CHARS);
    let emoji = "\u{1F44D}\u{FE0F}\u{FE0F}";
    assert_eq!(normalise(emoji).unwrap().as_str(), "\u{1F44D}\u{FE0F}");
}

#[test]
fn notices_carry_their_documented_arguments() {
    assert_eq!(notice::joined(3).args.to_vec(), vec![3]);
    let left = notice::left(4, LeaveReason::Timeout);
    assert_eq!((left.code, left.args.to_vec()), (NoticeCode::Leave, vec![4, 1]));
    assert_eq!(notice::kicked(2, None).args.to_vec(), vec![2, 0]);
    assert_eq!(notice::kicked(2, Some(0)).args.to_vec(), vec![2, u32::MAX]);
    assert_eq!(notice::kicked(2, Some(15)).args.to_vec(), vec![2, 15]);
    assert_eq!(notice::muted(1, true).args.to_vec(), vec![1, 1]);
    let vote = notice::vote(VoteKind::Kick, 5, notice::VoteNotice::Passed);
    assert_eq!((vote.code, vote.args.to_vec()), (NoticeCode::Vote, vec![0, 5, 1]));
    assert_eq!(notice::result(6, 1, 90_000).args.to_vec(), vec![6, 1, 90_000]);
}

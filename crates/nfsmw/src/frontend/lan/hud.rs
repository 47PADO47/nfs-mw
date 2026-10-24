//! The session's HUD pieces while the world has the screen (MP-40, 27, 41, 22): the chat and notice feed, the running
//! vote, the start countdown and the spectator's panel (who the camera follows, and the order of the racers). Plain data
//! built each frame; `view.rs` draws it.

use nfsmw_net::client_session::LobbyView;
use nfsmw_net::message::{Role, VoteKind};

use super::feed::FeedKind;
use super::menu::{LanMenu, countdown_text};
use super::results::ordered;
use super::text::{slot_name, votes};
use super::{NetFrontend, NetStatus};
use crate::i18n::fmt::ordinal;
use crate::i18n::{self, Args, t};

/// Seconds a feed line stays, and the last of them it fades over.
pub const FEED_SECONDS: f32 = 8.0;
const FADE_SECONDS: f32 = 2.0;
/// Lines the feed shows at once.
pub const FEED_LINES: usize = 6;

/// What the spectator sees besides the world.
#[derive(Clone, Debug, PartialEq)]
pub struct SpectatorPanel {
    /// `Following Ann`, or `Free camera`.
    pub following: String,
    /// The racers: in the last result's order when there is one, else by slot.
    pub board: Vec<String>,
}

/// The HUD pieces of this frame.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HudView {
    /// The feed's recent lines with their opacity, 0 to 1.
    pub feed: Vec<(FeedKind, String, f32)>,
    pub vote: Option<String>,
    pub countdown: Option<String>,
    pub spectator: Option<SpectatorPanel>,
}

/// The HUD of a session whose world has the screen; `None` with no session or while the menu covers the screen.
pub fn hud_view(menu: &LanMenu, net: &dyn NetFrontend) -> Option<HudView> {
    let status = net.status();
    if !matches!(status, NetStatus::Playing | NetStatus::Spectating) || menu.visible(&status) {
        return None;
    }
    let lobby = net.lobby();
    let feed = menu
        .feed
        .recent(FEED_SECONDS, FEED_LINES)
        .into_iter()
        .map(|line| (line.kind, line.text.clone(), ((FEED_SECONDS - line.age) / FADE_SECONDS).clamp(0.0, 1.0)))
        .collect();
    let server_tick = net.lead().server_tick;
    Some(HudView {
        feed,
        vote: lobby.and_then(|lobby| vote_text(lobby, server_tick)),
        countdown: lobby.and_then(|lobby| countdown_text(lobby, server_tick)),
        spectator: (status == NetStatus::Spectating).then(|| spectator(menu, net)),
    })
}

/// The key that votes for the running vote while the world has the screen.
pub const VOTE_KEY: &str = "F7";

/// Votes for the running vote's first option (the one a kick, restart or skip asks about), as the vote key does in a
/// race. `false` when no vote runs or the session did not take it.
pub fn cast_vote(net: &mut dyn NetFrontend) -> bool {
    let Some((kind, option)) = net.lobby().and_then(|lobby| {
        let vote = lobby.vote.as_ref()?;
        Some((vote.kind, vote.options.first()?.id))
    }) else {
        return false;
    };
    net.vote(kind, option)
}

/// `Vote: kick Ann  2 votes  20 s left  (F7 to vote)`.
fn vote_text(lobby: &LobbyView, server_tick: Option<f64>) -> Option<String> {
    let vote = lobby.vote.as_ref()?;
    let option = vote.options.first()?;
    let what = match vote.kind {
        VoteKind::Kick => t!("lan-hud-vote-kick", name = slot_name(Some(lobby), option.id)),
        VoteKind::Restart => vote_words("lan-hud-vote-restart"),
        VoteKind::ChangeMode => vote_words("lan-hud-vote-change-mode"),
        VoteKind::ChangeTrack => vote_words("lan-hud-vote-change-track"),
        VoteKind::SkipResults => vote_words("lan-hud-vote-skip-results"),
    };
    let mut text = t!("lan-hud-vote", what = what, votes = votes(option.count));
    if let Some(now) = server_tick {
        let left = ((f64::from(vote.ends_tick) - now) / 60.0).ceil().max(0.0);
        text.push_str(&format!("  {}", t!("lan-hud-vote-left", seconds = left as i64)));
    }
    text.push_str(&format!("  {}", t!("lan-hud-vote-key", key = VOTE_KEY)));
    Some(text)
}

fn vote_words(id: &str) -> String {
    i18n::text(id, &Args::new())
}

fn spectator(menu: &LanMenu, net: &dyn NetFrontend) -> SpectatorPanel {
    let lobby = net.lobby();
    let following = match net.follow_slot() {
        Some(slot) => t!("lan-hud-following", name = slot_name(lobby, u32::from(slot))),
        None => t!("lan-hud-free-camera"),
    };
    let board = match menu.results.as_ref().or(net.last_results()) {
        Some(results) => ordered(results)
            .iter()
            .map(|row| format!("{}  {}", ordinal(u32::from(row.place)), slot_name(lobby, u32::from(row.slot))))
            .collect(),
        None => lobby
            .map(|lobby| {
                let racers = lobby.players.iter().filter(|row| row.role != Role::Spectator);
                racers.map(|row| row.name.as_str().to_owned()).collect()
            })
            .unwrap_or_default(),
    };
    SpectatorPanel { following, board }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lan::mock::MockNetFrontend;
    use nfsmw_net::message::{List, VoteOption, VoteState};

    #[test]
    fn no_hud_without_a_world() {
        let menu = LanMenu::default();
        let net = MockNetFrontend::in_lobby(&["Host"], 0);
        assert_eq!(hud_view(&menu, &net), None);
    }

    #[test]
    fn the_feed_fades_and_the_vote_counts_down() {
        let mut menu = LanMenu::default();
        menu.feed.push(FeedKind::Chat, "Ann: go".into());
        menu.feed.tick(7.0);
        menu.feed.push(FeedKind::Notice, "Bob joined".into());
        let mut net = MockNetFrontend::in_lobby(&["Host", "Ann", "Bob"], 0);
        net.status = Some(NetStatus::Playing);
        net.lead.server_tick = Some(600.0);
        net.lobby.as_mut().unwrap().vote = Some(VoteState {
            kind: VoteKind::Kick,
            options: List::new(vec![VoteOption { id: 2, count: 1 }]).unwrap(),
            ends_tick: 1800,
        });
        let hud = hud_view(&menu, &net).unwrap();
        assert_eq!(hud.feed.len(), 2);
        assert_eq!(hud.feed[0].1, "Ann: go");
        assert!((hud.feed[0].2 - 0.5).abs() < 1e-4, "half faded");
        assert_eq!(hud.feed[1].2, 1.0);
        assert_eq!(hud.vote.as_deref(), Some("Vote: kick Bob  1 vote  20 s left  (F7 to vote)"));
        assert_eq!(hud.spectator, None);
    }

    #[test]
    fn the_vote_key_votes_for_the_running_vote_only() {
        use crate::frontend::lan::mock::Call;
        let mut net = MockNetFrontend::in_lobby(&["Host", "Ann"], 0);
        net.status = Some(NetStatus::Playing);
        assert!(!cast_vote(&mut net), "no vote runs");
        net.lobby.as_mut().unwrap().vote = Some(VoteState {
            kind: VoteKind::Kick,
            options: List::new(vec![VoteOption { id: 1, count: 0 }]).unwrap(),
            ends_tick: 1800,
        });
        assert!(cast_vote(&mut net));
        assert_eq!(net.take_calls(), [Call::Vote(VoteKind::Kick, 1)]);
    }

    #[test]
    fn a_spectator_sees_who_is_followed_and_the_racers() {
        let menu = LanMenu::default();
        let mut net = MockNetFrontend::in_lobby(&["Host", "Ann", "Me"], 2);
        net.status = Some(NetStatus::Spectating);
        net.follow = Some(1);
        net.lobby.as_mut().unwrap().players[2].role = Role::Spectator;
        let panel = hud_view(&menu, &net).unwrap().spectator.unwrap();
        assert_eq!(panel.following, "Following Ann");
        assert_eq!(panel.board, ["Host", "Ann"]);
    }
}

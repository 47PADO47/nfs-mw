//! Path validation for a server-side peer (security.md section 4.4): a peer whose sealed packets start to arrive from
//! another address (a NAT rebinding, a roaming device) keeps its session only after it proves it can be reached there.
//!
//! The packet that comes from the new address has to open under the peer's keys and carry a packet number the window
//! has not seen, so a copy of an old packet changes nothing. The connection still sends to the old address. A
//! challenge with 8 random bytes goes to the new one, in a packet with nothing else in it, and only a path response
//! with those bytes, read from that same address, moves the peer. A spoofer who has copied a live packet gets a
//! challenge sent to the address it forged and cannot answer it, since the token is sealed.
//!
//! The cost to a third party is bounded: a candidate address gets at most [`MAX_PROBES`] challenges of one small
//! packet each, one candidate at a time, and after a candidate fails or the peer moves, the next one waits
//! [`QUIET_MS`]. Packets from unvalidated addresses are checked at most [`FOREIGN_PER_WINDOW`] times a second.

use std::net::SocketAddr;

use crate::conn::PATH_TOKEN_LEN;

/// Challenges sent to one candidate address.
const MAX_PROBES: u8 = 3;
/// Time between challenges to the same candidate, milliseconds.
const PROBE_EVERY_MS: u64 = 250;
/// How long a candidate has to answer, milliseconds.
const CANDIDATE_MS: u64 = 3_000;
/// How long after a candidate ended (failed or confirmed) before another one is tried, milliseconds.
const QUIET_MS: u64 = 1_000;
/// Packets from addresses the peer is not known at that are opened per [`FOREIGN_WINDOW_MS`].
const FOREIGN_PER_WINDOW: u32 = 20;
/// The window of [`FOREIGN_PER_WINDOW`], milliseconds.
const FOREIGN_WINDOW_MS: u64 = 1_000;

/// An address that is being asked to prove itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Candidate {
    addr: SocketAddr,
    token: [u8; PATH_TOKEN_LEN],
    next_probe_ms: u64,
    probes: u8,
    expires_ms: u64,
}

/// The state of a peer's path validation.
#[derive(Debug, Default)]
pub(crate) struct PathProbe {
    candidate: Option<Candidate>,
    quiet_until_ms: u64,
    window_start_ms: u64,
    window_count: u32,
}

impl PathProbe {
    /// Whether a packet from an unvalidated address may be opened now: a cheap limit before any cryptography.
    pub(crate) fn admit_foreign(&mut self, now_ms: u64) -> bool {
        if now_ms.saturating_sub(self.window_start_ms) >= FOREIGN_WINDOW_MS {
            self.window_start_ms = now_ms;
            self.window_count = 0;
        }
        if self.window_count >= FOREIGN_PER_WINDOW {
            return false;
        }
        self.window_count += 1;
        true
    }

    /// An authenticated packet came from `addr`, which is not the peer's address. Starts asking `addr` to prove itself
    /// unless a candidate is being asked already or the last one ended a moment ago. `token` is made only when needed.
    pub(crate) fn offer(
        &mut self,
        now_ms: u64,
        addr: SocketAddr,
        token: impl FnOnce() -> Option<[u8; PATH_TOKEN_LEN]>,
    ) {
        self.expire(now_ms);
        if self.candidate.is_some() || now_ms < self.quiet_until_ms {
            return;
        }
        let Some(token) = token() else { return };
        self.candidate =
            Some(Candidate { addr, token, next_probe_ms: now_ms, probes: 0, expires_ms: now_ms + CANDIDATE_MS });
    }

    /// The challenge to send now, as the address and the token, if one is due. Each call that returns one counts it.
    pub(crate) fn probe_due(&mut self, now_ms: u64) -> Option<(SocketAddr, [u8; PATH_TOKEN_LEN])> {
        self.expire(now_ms);
        let candidate = self.candidate.as_mut()?;
        if now_ms < candidate.next_probe_ms || candidate.probes >= MAX_PROBES {
            return None;
        }
        candidate.probes += 1;
        candidate.next_probe_ms = now_ms + PROBE_EVERY_MS;
        Some((candidate.addr, candidate.token))
    }

    /// A path response with `token` was read from `from`. True when it proves the candidate, which then ends.
    pub(crate) fn confirm(&mut self, now_ms: u64, from: SocketAddr, token: &[u8; PATH_TOKEN_LEN]) -> bool {
        let proves = self.candidate.is_some_and(|candidate| candidate.addr == from && candidate.token == *token);
        if proves {
            self.candidate = None;
            self.quiet_until_ms = now_ms + QUIET_MS;
        }
        proves
    }

    /// Ends the candidate that ran out of time.
    fn expire(&mut self, now_ms: u64) {
        if self.candidate.is_some_and(|candidate| now_ms >= candidate.expires_ms) {
            self.candidate = None;
            self.quiet_until_ms = now_ms + QUIET_MS;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    const TOKEN: [u8; PATH_TOKEN_LEN] = [7; PATH_TOKEN_LEN];

    #[test]
    fn a_candidate_is_challenged_a_few_times_and_confirmed_only_by_its_own_address_and_token() {
        let mut path = PathProbe::default();
        path.offer(0, addr(1), || Some(TOKEN));
        assert_eq!(path.probe_due(0), Some((addr(1), TOKEN)));
        assert_eq!(path.probe_due(100), None, "the next challenge waits");
        assert_eq!(path.probe_due(250), Some((addr(1), TOKEN)));
        assert!(!path.confirm(300, addr(2), &TOKEN), "another address does not prove it");
        assert!(!path.confirm(300, addr(1), &[8; PATH_TOKEN_LEN]), "another token does not");
        assert!(path.confirm(300, addr(1), &TOKEN));
        assert_eq!(path.probe_due(600), None, "a confirmed candidate is done");
    }

    #[test]
    fn challenges_stop_after_the_limit_and_the_candidate_expires() {
        let mut path = PathProbe::default();
        path.offer(0, addr(1), || Some(TOKEN));
        let sent = (0..10).filter(|step| path.probe_due(step * 250).is_some()).count();
        assert_eq!(sent, usize::from(MAX_PROBES));
        assert_eq!(path.probe_due(CANDIDATE_MS), None);
        assert!(!path.confirm(CANDIDATE_MS, addr(1), &TOKEN), "an answer after the time is too late");
    }

    #[test]
    fn one_candidate_at_a_time_and_a_quiet_time_between_them() {
        let mut path = PathProbe::default();
        path.offer(0, addr(1), || Some(TOKEN));
        path.offer(10, addr(2), || Some([9; PATH_TOKEN_LEN]));
        assert_eq!(path.probe_due(10), Some((addr(1), TOKEN)), "the first stays");
        assert!(path.confirm(20, addr(1), &TOKEN));
        path.offer(500, addr(2), || Some([9; PATH_TOKEN_LEN]));
        assert_eq!(path.probe_due(500), None, "a new candidate waits out the quiet time");
        path.offer(1_100, addr(2), || Some([9; PATH_TOKEN_LEN]));
        assert_eq!(path.probe_due(1_100), Some((addr(2), [9; PATH_TOKEN_LEN])));
    }

    #[test]
    fn unvalidated_packets_are_opened_at_a_limited_rate() {
        let mut path = PathProbe::default();
        let opened = (0..100).filter(|_| path.admit_foreign(5)).count();
        assert_eq!(opened, FOREIGN_PER_WINDOW as usize);
        assert!(path.admit_foreign(5 + FOREIGN_WINDOW_MS), "a new window starts");
    }

    #[test]
    fn no_token_means_no_candidate() {
        let mut path = PathProbe::default();
        path.offer(0, addr(1), || None);
        assert_eq!(path.probe_due(0), None);
    }
}

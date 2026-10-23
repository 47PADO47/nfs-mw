//! LAN discovery from the game: a broadcast scan on the discovery port (`blackbox_net_core::endpoint::DiscoveryScan`),
//! its answers kept once per server, and the line `--discover` prints for each.

use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use blackbox_net_core::discovery::FLAG_IN_PROGRESS;
use blackbox_net_core::endpoint::{DiscoveryScan, Found, ScanConfig};
use nfsmw_net::message::Mode;
use nfsmw_net::version::PROTO_MAJOR;

/// The servers a scan found, one entry per game address, newest answer kept.
#[derive(Debug, Default)]
pub struct Servers {
    found: Vec<Found>,
}

impl Servers {
    /// Takes `found`, replacing an earlier answer from the same server.
    pub fn note(&mut self, found: Found) {
        match self.found.iter_mut().find(|known| known.game_addr == found.game_addr) {
            Some(known) => *known = found,
            None => self.found.push(found),
        }
    }
}

/// The scan of the LAN for servers on `port` (their discovery port is `port + 1`), for `duration`.
pub fn scan_config(port: u16, duration: Duration) -> ScanConfig {
    let mut config = ScanConfig::broadcast(port.saturating_add(1), PROTO_MAJOR);
    config.duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX).max(1);
    config
}

/// Scans the LAN and waits for the scan to end.
pub fn scan(port: u16, duration: Duration) -> io::Result<Vec<Found>> {
    let mut scan = DiscoveryScan::start(scan_config(port, duration))?;
    let mut servers = Servers::default();
    loop {
        while let Some(found) = scan.poll() {
            servers.note(found);
        }
        if scan.is_finished() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    while let Some(found) = scan.poll() {
        servers.note(found);
    }
    Ok(servers.found)
}

/// One line about a server: its address, name, players, mode and whether a session runs.
pub fn describe(found: &Found) -> String {
    let reply = &found.reply;
    let mode = Mode::from_code(reply.mode).map_or_else(|| format!("mode {}", reply.mode), |mode| format!("{mode:?}"));
    let running = match reply.flags & FLAG_IN_PROGRESS != 0 {
        true => ", in progress",
        false => "",
    };
    format!(
        "{}  \"{}\"  {}/{} players  {mode}{running}",
        address(found.game_addr),
        reply.name,
        reply.players,
        reply.max_players
    )
}

fn address(addr: SocketAddr) -> String {
    addr.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_net_core::discovery::Reply;

    fn found(port: u16, players: u8) -> Found {
        let reply = Reply {
            proto_major: PROTO_MAJOR,
            nonce: 0,
            proto_minor: 0,
            key_hash: [0; 8],
            game_port: port,
            players,
            max_players: 16,
            flags: FLAG_IN_PROGRESS,
            mode: Mode::FreeRoam.code(),
            data_id: [0; 4],
            name: "Rockport".into(),
        };
        let game_addr = SocketAddr::from(([192, 168, 1, 5], port));
        Found { from: SocketAddr::from(([192, 168, 1, 5], port + 1)), game_addr, reply }
    }

    #[test]
    fn a_server_is_listed_once_with_its_newest_answer() {
        let mut servers = Servers::default();
        servers.note(found(9750, 1));
        servers.note(found(9760, 2));
        servers.note(found(9750, 3));
        let players: Vec<u8> = servers.found.iter().map(|found| found.reply.players).collect();
        assert_eq!(players, [3, 2]);
    }

    #[test]
    fn the_line_names_the_address_name_players_and_mode() {
        let line = describe(&found(9750, 3));
        assert!(line.starts_with("192.168.1.5:9750"), "{line}");
        assert!(
            line.contains("\"Rockport\"") && line.contains("3/16 players") && line.contains("in progress"),
            "{line}"
        );
    }

    #[test]
    fn the_scan_asks_the_discovery_port() {
        let config = scan_config(9750, Duration::from_millis(500));
        assert_eq!(config.targets[0].port(), 9751);
        assert_eq!(config.duration_ms, 500);
    }
}

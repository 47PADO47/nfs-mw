//! The `net.*` console commands (`docs/specs/multiplayer/integration.md` section 2) as plain data: text in,
//! [`NetCommand`] out. `net_cmd.rs` runs them through the game's `NetFrontend`.

use std::net::IpAddr;

use nfsmw_net::netsim::NetSim;

/// A `net.*` command.
#[derive(Debug, Clone, PartialEq)]
pub enum NetCommand {
    /// `net.host [room name]`: a listen server with the settings' port and the LAN defaults.
    Host { room: Option<String> },
    /// `net.connect <ip[:port]> [password]`: the port defaults to `net_port`.
    Connect { ip: IpAddr, port: Option<u16>, password: Option<String> },
    /// `net.disconnect`: leave the session (a host closes its server).
    Disconnect,
    /// `net.stats`: the overlay's network line.
    Stats,
    /// `net.sim <loss %> <rtt ms> <jitter ms>` or `net.sim off`.
    Sim(NetSim),
    /// `net.kick <slot>` (host or admin).
    Kick(u8),
    /// `net.say <text>`.
    Say(String),
    /// `net.lead`: the estimated server tick and the input lead.
    Lead,
    /// `net.scan`: a LAN discovery scan, then the servers found.
    Scan,
    /// `net.servers`: the servers the last scan found.
    Servers,
    /// `net.status`: where the session is.
    Status,
}

/// The `net.*` commands for `help` and Tab: usage and description.
pub const NET_COMMANDS: [(&str, &str); 11] = [
    ("net.host [room name]", "host a LAN game with the net_* settings"),
    ("net.connect <ip[:port]> [password]", "join a LAN game by address"),
    ("net.disconnect", "leave the LAN game (a host closes it)"),
    ("net.stats", "show the network line: rtt, jitter, loss, snapshot age, corrections, bandwidth"),
    ("net.sim <loss %> <rtt ms> <jitter ms>|off", "simulate a bad network on the session's link"),
    ("net.kick <slot>", "host or admin: remove a player"),
    ("net.say <text>", "chat to everybody in the session"),
    ("net.lead", "show the estimated server tick and the input lead"),
    ("net.scan", "look for LAN games"),
    ("net.servers", "list the LAN games the last scan found"),
    ("net.status", "show where the session is"),
];

/// Parses `name` (lower case) and its arguments when `name` is a `net.*` command; `None` for any other name.
pub fn parse(name: &str, args: &[&str]) -> Option<Result<NetCommand, String>> {
    let rest = name.strip_prefix("net.")?;
    Some(command(rest, args))
}

fn command(name: &str, args: &[&str]) -> Result<NetCommand, String> {
    match (name, args) {
        ("host", []) => Ok(NetCommand::Host { room: None }),
        ("host", words) => Ok(NetCommand::Host { room: Some(words.join(" ")) }),
        ("connect", [address]) => connect(address, None),
        ("connect", [address, password]) => connect(address, Some(password)),
        ("connect", _) => Err("usage: net.connect <ip[:port]> [password]".into()),
        ("disconnect" | "leave", []) => Ok(NetCommand::Disconnect),
        ("stats", []) => Ok(NetCommand::Stats),
        ("sim", words) => NetSim::parse(&words.join(" ")).map(NetCommand::Sim).map_err(|e| e.to_string()),
        ("kick", [slot]) => kick(slot),
        ("kick", _) => Err("usage: net.kick <slot>".into()),
        ("say", []) => Err("usage: net.say <text>".into()),
        ("say", words) => Ok(NetCommand::Say(words.join(" "))),
        ("lead", []) => Ok(NetCommand::Lead),
        ("scan", []) => Ok(NetCommand::Scan),
        ("servers", []) => Ok(NetCommand::Servers),
        ("status", []) => Ok(NetCommand::Status),
        (known, _) if is_listed(known) => Err(format!("net.{known} takes no arguments")),
        (other, _) => Err(format!("unknown command net.{other} (type help)")),
    }
}

/// Whether `net.<name>` is one of [`NET_COMMANDS`].
fn is_listed(name: &str) -> bool {
    let full = format!("net.{name}");
    NET_COMMANDS.iter().any(|(usage, _)| usage.split_whitespace().next() == Some(full.as_str()))
}

fn kick(slot: &str) -> Result<NetCommand, String> {
    let slot: u8 = slot.parse().map_err(|_| format!("{slot:?} is not a slot"))?;
    if !nfsmw_net::message::is_slot(slot) {
        return Err(format!("slot {slot} is out of range"));
    }
    Ok(NetCommand::Kick(slot))
}

/// `1.2.3.4`, `1.2.3.4:9760`, `[::1]:9760`, `::1` or `localhost[:port]`.
fn connect(address: &str, password: Option<&str>) -> Result<NetCommand, String> {
    let (ip, port) = address_of(address)?;
    Ok(NetCommand::Connect { ip, port, password: password.map(str::to_owned) })
}

/// The IP and the port, if one was given, of an address as a player types it.
pub fn address_of(address: &str) -> Result<(IpAddr, Option<u16>), String> {
    let bad = || format!("{address:?} is not an address (an IP, optionally with :port)");
    if let Ok(ip) = address.parse::<IpAddr>() {
        return Ok((ip, None));
    }
    if let Ok(socket) = address.parse::<std::net::SocketAddr>() {
        return Ok((socket.ip(), Some(socket.port())));
    }
    let (host, port) = match address.rsplit_once(':') {
        Some((host, port)) => (host, Some(port.parse::<u16>().map_err(|_| bad())?)),
        None => (address, None),
    };
    if !host.eq_ignore_ascii_case("localhost") {
        return Err(bad());
    }
    Ok((IpAddr::from([127, 0, 0, 1]), port))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(line: &str) -> NetCommand {
        let mut words = line.split_whitespace();
        let name = words.next().unwrap();
        let args: Vec<&str> = words.collect();
        parse(name, &args).expect("a net command").expect("parses")
    }

    fn err(line: &str) -> String {
        let mut words = line.split_whitespace();
        let name = words.next().unwrap();
        let args: Vec<&str> = words.collect();
        parse(name, &args).expect("a net command").expect_err("refused")
    }

    #[test]
    fn other_names_are_not_net_commands() {
        assert!(parse("car", &[]).is_none());
        assert!(parse("network", &[]).is_none());
    }

    #[test]
    fn host_takes_an_optional_room_name() {
        assert_eq!(ok("net.host"), NetCommand::Host { room: None });
        assert_eq!(ok("net.host Friday night"), NetCommand::Host { room: Some("Friday night".into()) });
    }

    #[test]
    fn connect_takes_an_address_with_or_without_a_port_and_a_password() {
        let ip = IpAddr::from([192, 168, 1, 5]);
        assert_eq!(ok("net.connect 192.168.1.5"), NetCommand::Connect { ip, port: None, password: None });
        assert_eq!(
            ok("net.connect 192.168.1.5:9760 hunter2"),
            NetCommand::Connect { ip, port: Some(9760), password: Some("hunter2".into()) }
        );
        let local = IpAddr::from([127, 0, 0, 1]);
        assert_eq!(
            ok("net.connect localhost:9751"),
            NetCommand::Connect { ip: local, port: Some(9751), password: None }
        );
        assert_eq!(
            ok("net.connect ::1"),
            NetCommand::Connect { ip: "::1".parse().unwrap(), port: None, password: None }
        );
        assert!(err("net.connect").starts_with("usage"));
        assert!(err("net.connect example.com").contains("not an address"));
        assert!(err("net.connect 1.2.3.4:99999").contains("not an address"));
    }

    #[test]
    fn sim_takes_three_numbers_or_off() {
        assert_eq!(ok("net.sim 2 40 5"), NetCommand::Sim(NetSim::new(2.0, 40, 5).unwrap()));
        assert_eq!(ok("net.sim 2% 40ms 5ms"), NetCommand::Sim(NetSim::new(2.0, 40, 5).unwrap()));
        assert_eq!(ok("net.sim off"), NetCommand::Sim(NetSim::OFF));
        assert!(err("net.sim 2 40").starts_with("usage"));
        assert!(err("net.sim 90 0 0").contains("loss"));
    }

    #[test]
    fn kick_takes_a_slot_in_range() {
        assert_eq!(ok("net.kick 3"), NetCommand::Kick(3));
        assert!(err("net.kick").starts_with("usage"));
        assert!(err("net.kick 40").contains("out of range"));
        assert!(err("net.kick me").contains("not a slot"));
    }

    #[test]
    fn say_keeps_the_whole_line() {
        assert_eq!(ok("net.say police   behind me"), NetCommand::Say("police behind me".into()));
        assert!(err("net.say").starts_with("usage"));
    }

    #[test]
    fn the_plain_commands_take_no_arguments() {
        assert_eq!(ok("net.disconnect"), NetCommand::Disconnect);
        assert_eq!(ok("net.leave"), NetCommand::Disconnect);
        assert_eq!(ok("net.stats"), NetCommand::Stats);
        assert_eq!(ok("net.lead"), NetCommand::Lead);
        assert_eq!(ok("net.scan"), NetCommand::Scan);
        assert_eq!(ok("net.servers"), NetCommand::Servers);
        assert_eq!(ok("net.status"), NetCommand::Status);
        assert_eq!(err("net.stats now"), "net.stats takes no arguments");
        assert!(err("net.frobnicate").starts_with("unknown command net.frobnicate"));
    }
}

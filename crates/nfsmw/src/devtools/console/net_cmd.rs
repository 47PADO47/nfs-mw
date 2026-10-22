//! Running a [`NetCommand`] through the game's `NetFrontend`: the same seam the LAN screens use, so a script or the
//! console drives a session exactly as the menus do.

use std::net::SocketAddr;

use nfsmw_net::message::AdminCommand;

use super::net_parse::NetCommand;
use crate::frontend::lan::host_options::{default_room, default_settings, host_options};
use crate::frontend::lan::text::{self, server_line, status_text};
use crate::frontend::lan::{LeadInfo, NetFrontend, NetStatus};
use crate::settings::Settings;

/// Runs `command` on `net` (`None` when the game has no network) with the `net_*` settings of `settings`.
pub fn run(command: NetCommand, net: Option<&mut dyn NetFrontend>, settings: &Settings) -> Result<String, String> {
    let net = net.ok_or("there is no network in this run")?;
    match command {
        NetCommand::Host { room } => host(net, settings, room),
        NetCommand::Connect { ip, port, password } => {
            let addr = SocketAddr::new(ip, port.unwrap_or(settings.net.port));
            net.connect(addr, password.as_deref())?;
            Ok(format!("connecting to {addr}"))
        }
        NetCommand::Disconnect => {
            if net.status() == NetStatus::Idle {
                return Err("not in a session".into());
            }
            net.leave();
            Ok("left the session".into())
        }
        NetCommand::Stats => net.stats_line().map(|line| line.to_string()).ok_or_else(|| "not in a session".into()),
        NetCommand::Sim(sim) => {
            net.set_net_sim(sim)?;
            Ok(sim.to_string())
        }
        NetCommand::Kick(slot) => {
            let name = text::slot_name(net.lobby(), u32::from(slot));
            match net.admin(AdminCommand::Kick { slot }) {
                true => Ok(format!("asked the server to kick {name}")),
                false => Err("not in a session".into()),
            }
        }
        NetCommand::Say(line) => match net.say(&line) {
            Ok(true) => Ok(String::new()),
            Ok(false) => Err("not in a session".into()),
            Err(e) => Err(format!("the line was refused: {e:?}")),
        },
        NetCommand::Lead => Ok(lead_text(net.lead())),
        NetCommand::Scan => {
            net.scan();
            Ok("looking for LAN games; net.servers lists them".into())
        }
        NetCommand::Servers => Ok(servers_text(&*net)),
        NetCommand::Status => Ok(status_text(&net.status())),
    }
}

fn host(net: &mut dyn NetFrontend, settings: &Settings, room: Option<String>) -> Result<String, String> {
    let room = room.unwrap_or_else(|| default_room(settings.net.name.as_str()));
    let session = default_settings(&room)?;
    let options = host_options(&settings.net, session, "", crate::scenes::world::DEFAULT_CAR)?;
    let port = options.port;
    net.host(options)?;
    Ok(format!("hosting {room:?} on port {port}"))
}

/// `server tick 1234.5  lead 4 ticks  rtt 3 ms`; a value not measured yet reads `-`.
pub fn lead_text(lead: LeadInfo) -> String {
    let tick = lead.server_tick.map_or_else(|| "-".to_owned(), |tick| format!("{tick:.1}"));
    let ticks = lead.lead_ticks.map_or_else(|| "-".to_owned(), |ticks| format!("{ticks} ticks"));
    let rtt = lead.rtt_ms.map_or_else(|| "-".to_owned(), |rtt| format!("{rtt:.0} ms"));
    format!("server tick {tick}  lead {ticks}  rtt {rtt}")
}

fn servers_text(net: &dyn NetFrontend) -> String {
    if net.servers().is_empty() {
        return "no LAN games found (net.scan looks again)".into();
    }
    net.servers()
        .iter()
        .map(|entry| {
            let (name, value) = server_line(entry);
            format!("  {}  {name}  {value}", entry.addr)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lan::ServerEntry;
    use crate::frontend::lan::mock::{Call, MockNetFrontend};
    use nfsmw_net::netsim::NetSim;
    use nfsmw_net::netstats::NetLine;

    fn settings() -> Settings {
        Settings::from(crate::settings::Partial::default())
    }

    fn run_on(net: &mut MockNetFrontend, command: NetCommand) -> Result<String, String> {
        run(command, Some(net as &mut dyn NetFrontend), &settings())
    }

    #[test]
    fn without_a_network_every_command_says_so() {
        assert_eq!(run(NetCommand::Stats, None, &settings()), Err("there is no network in this run".into()));
    }

    #[test]
    fn host_uses_the_settings_and_the_lan_defaults() {
        let mut net = MockNetFrontend::default();
        assert_eq!(
            run_on(&mut net, NetCommand::Host { room: None }),
            Ok("hosting \"Player's game\" on port 9750".into())
        );
        let calls: [Call; 1] = net.take_calls().try_into().unwrap();
        let [Call::Host(options)] = calls else { panic!("one host call") };
        assert_eq!(options.settings.name.as_str(), "Player's game");
        assert_eq!((options.port, options.settings.max_players), (9750, 8));
        net.fail = Some("port 9750 is in use".into());
        assert_eq!(run_on(&mut net, NetCommand::Host { room: Some("x".into()) }), Err("port 9750 is in use".into()));
    }

    #[test]
    fn connect_fills_in_the_settings_port() {
        let mut net = MockNetFrontend::default();
        let ip = "10.0.0.2".parse().unwrap();
        run_on(&mut net, NetCommand::Connect { ip, port: None, password: Some("pw".into()) }).unwrap();
        run_on(&mut net, NetCommand::Connect { ip, port: Some(9800), password: None }).unwrap();
        assert_eq!(
            net.take_calls(),
            [
                Call::Connect("10.0.0.2:9750".parse().unwrap(), Some("pw".into())),
                Call::Connect("10.0.0.2:9800".parse().unwrap(), None)
            ]
        );
    }

    #[test]
    fn session_commands_need_a_session() {
        let mut idle = MockNetFrontend::default();
        assert_eq!(run_on(&mut idle, NetCommand::Disconnect), Err("not in a session".into()));
        assert_eq!(run_on(&mut idle, NetCommand::Kick(1)), Err("not in a session".into()));
        assert_eq!(run_on(&mut idle, NetCommand::Stats), Err("not in a session".into()));
        let mut net = MockNetFrontend::in_lobby(&["Host", "Ann"], 0);
        assert_eq!(run_on(&mut net, NetCommand::Kick(1)), Ok("asked the server to kick Ann".into()));
        assert_eq!(run_on(&mut net, NetCommand::Say("hi all".into())), Ok(String::new()));
        assert_eq!(run_on(&mut net, NetCommand::Disconnect), Ok("left the session".into()));
        assert_eq!(
            net.take_calls(),
            [Call::Admin(AdminCommand::Kick { slot: 1 }), Call::Say("hi all".into()), Call::Leave]
        );
    }

    #[test]
    fn stats_sim_lead_and_status_report() {
        let mut net = MockNetFrontend::in_lobby(&["Host"], 0);
        net.line = Some(NetLine { rtt_ms: Some(3.0), ..NetLine::default() });
        assert!(run_on(&mut net, NetCommand::Stats).unwrap().starts_with("net rtt 3 ms"));
        let sim = NetSim::new(2.0, 40, 5).unwrap();
        assert_eq!(run_on(&mut net, NetCommand::Sim(sim)), Ok("net.sim 2 40 5".into()));
        net.lead = LeadInfo { server_tick: Some(120.5), lead_ticks: Some(4), rtt_ms: None };
        assert_eq!(run_on(&mut net, NetCommand::Lead), Ok("server tick 120.5  lead 4 ticks  rtt -".into()));
        assert_eq!(run_on(&mut net, NetCommand::Status), Ok("in the lobby".into()));
        assert!(run_on(&mut net, NetCommand::Say(String::new())).unwrap_err().starts_with("the line was refused"));
    }

    #[test]
    fn scan_then_servers_lists_what_answered() {
        let mut net = MockNetFrontend::default();
        assert!(run_on(&mut net, NetCommand::Scan).unwrap().starts_with("looking for LAN games"));
        assert_eq!(net.take_calls(), [Call::Scan]);
        assert!(run_on(&mut net, NetCommand::Servers).unwrap().starts_with("no LAN games found"));
        net.servers.push(ServerEntry {
            addr: "10.0.0.2:9750".parse().unwrap(),
            name: "Room".into(),
            mode: Some(nfsmw_net::message::Mode::FreeRoam),
            players: 1,
            max_players: 8,
            compatible: true,
            password: false,
            in_progress: false,
            ping_ms: None,
        });
        assert_eq!(run_on(&mut net, NetCommand::Servers), Ok("  10.0.0.2:9750  Room  Free Roam  1/8".into()));
    }
}

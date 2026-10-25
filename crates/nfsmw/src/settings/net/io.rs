//! Where the LAN settings come from: the config file's top-level keys, the environment and the command line. A value that
//! does not parse is reported and left unset, so a lower layer or the default stands.

use clap::Args;
use toml::{Table, Value};

use super::super::partial::parse_bool;
use super::{
    IdentityPath, NetLayer, NetName, PublicAddress, parse_interp_delay, parse_port, parse_public_address,
    parse_snapshot_hz, valid_name,
};

pub const NAME_KEY: &str = "net_name";
pub const PORT_KEY: &str = "net_port";
pub const SNAPSHOT_HZ_KEY: &str = "net_snapshot_hz";
pub const INTERP_DELAY_KEY: &str = "net_interp_delay";
pub const SHOW_STATS_KEY: &str = "net_show_stats";
pub const IDENTITY_FILE_KEY: &str = "net_identity_file";
pub const PUBLIC_ADDRESS_KEY: &str = "net_public_address";

pub const NAME_ENV: &str = "NFSMW_NET_NAME";
pub const PORT_ENV: &str = "NFSMW_NET_PORT";
pub const SNAPSHOT_HZ_ENV: &str = "NFSMW_NET_SNAPSHOT_HZ";
pub const INTERP_DELAY_ENV: &str = "NFSMW_NET_INTERP_DELAY";
pub const SHOW_STATS_ENV: &str = "NFSMW_NET_SHOW_STATS";
pub const IDENTITY_FILE_ENV: &str = "NFSMW_NET_IDENTITY_FILE";
pub const PUBLIC_ADDRESS_ENV: &str = "NFSMW_NET_PUBLIC_ADDRESS";

/// The keys of the config file (`origin` names the file in the warnings).
pub fn read(table: &Table, origin: &str) -> NetLayer {
    let text = |key: &str| -> Option<String> {
        let value = table.get(key)?;
        match value {
            Value::String(text) => Some(text.clone()),
            Value::Integer(n) => Some(n.to_string()),
            Value::Float(f) => Some(f.to_string()),
            Value::Boolean(b) => Some(b.to_string()),
            _ => warned(origin, key, "expected a value".into()),
        }
    };
    let parse = |key: &str, convert: &dyn Fn(&str) -> Result<(), String>| {
        let value = text(key)?;
        match convert(&value) {
            Ok(()) => Some(value),
            Err(e) => warned(origin, key, e),
        }
    };
    NetLayer {
        name: parse(NAME_KEY, &|v| valid_name(v).map(drop)).and_then(|v| NetName::new(&v).ok()),
        port: parse(PORT_KEY, &|v| parse_port(v).map(drop)).and_then(|v| parse_port(&v).ok()),
        snapshot_hz: parse(SNAPSHOT_HZ_KEY, &|v| parse_snapshot_hz(v).map(drop))
            .and_then(|v| parse_snapshot_hz(&v).ok()),
        interp_delay: parse(INTERP_DELAY_KEY, &|v| parse_interp_delay(v).map(drop))
            .and_then(|v| parse_interp_delay(&v).ok()),
        show_stats: parse(SHOW_STATS_KEY, &|v| parse_bool(v).map(drop)).and_then(|v| parse_bool(&v).ok()),
        identity_file: parse(IDENTITY_FILE_KEY, &|v| IdentityPath::new(v).map(drop))
            .and_then(|v| IdentityPath::new(&v).ok()),
        public_address: parse(PUBLIC_ADDRESS_KEY, &|v| parse_public_address(v).map(drop))
            .and_then(|v| parse_public_address(&v).ok()),
    }
}

fn warned<T>(origin: &str, key: &str, why: String) -> Option<T> {
    log::warn!("{origin}: ignoring `{key}`: {why}");
    None
}

/// The environment variables. A value that does not parse is ignored.
pub fn from_env(get: impl Fn(&str) -> Option<String>) -> NetLayer {
    let value = |name: &str| get(name).filter(|v| !v.trim().is_empty());
    NetLayer {
        name: value(NAME_ENV).and_then(|v| valid_name(&v).ok()),
        port: value(PORT_ENV).and_then(|v| parse_port(&v).ok()),
        snapshot_hz: value(SNAPSHOT_HZ_ENV).and_then(|v| parse_snapshot_hz(&v).ok()),
        interp_delay: value(INTERP_DELAY_ENV).and_then(|v| parse_interp_delay(&v).ok()),
        show_stats: value(SHOW_STATS_ENV).and_then(|v| parse_bool(&v).ok()),
        identity_file: value(IDENTITY_FILE_ENV).and_then(|v| IdentityPath::new(&v).ok()),
        public_address: value(PUBLIC_ADDRESS_ENV).and_then(|v| parse_public_address(&v).ok()),
    }
}

/// Puts the values `changes` sets into the config file's table, as top-level keys.
pub fn put_into(changes: &NetLayer, put: &mut impl FnMut(&str, Value)) {
    if let Some(name) = changes.name {
        put(NAME_KEY, Value::String(name.as_str().to_owned()));
    }
    if let Some(port) = changes.port {
        put(PORT_KEY, Value::Integer(i64::from(port)));
    }
    if let Some(hz) = changes.snapshot_hz {
        put(SNAPSHOT_HZ_KEY, Value::Integer(i64::from(hz)));
    }
    if let Some(delay) = changes.interp_delay {
        put(INTERP_DELAY_KEY, Value::Float(f64::from(delay)));
    }
    if let Some(show) = changes.show_stats {
        put(SHOW_STATS_KEY, Value::Boolean(show));
    }
    if let Some(path) = changes.identity_file {
        put(IDENTITY_FILE_KEY, Value::String(path.as_str().to_owned()));
    }
    if let Some(address) = changes.public_address {
        put(PUBLIC_ADDRESS_KEY, Value::String(address.as_str().to_owned()));
    }
}

/// The command-line options. Unset options leave the environment or the config file in charge.
#[derive(Debug, Clone, Default, Args)]
pub struct NetArgs {
    /// Your name in a LAN session [env NFSMW_NET_NAME; config `net_name`; default Player].
    #[arg(long, value_name = "NAME", value_parser = valid_name)]
    pub net_name: Option<NetName>,
    /// The game port a LAN host binds; discovery answers on the next one [env NFSMW_NET_PORT; default 9750].
    #[arg(long, value_name = "PORT", value_parser = parse_port)]
    pub net_port: Option<u16>,
    /// Snapshots per second a LAN host sends each player, 1 to 60 [env NFSMW_NET_SNAPSHOT_HZ; default 20].
    #[arg(long, value_name = "HZ", value_parser = parse_snapshot_hz)]
    pub net_snapshot_hz: Option<u8>,
    /// How far in the past remote cars are drawn, in snapshot intervals, 1 to 4 [env NFSMW_NET_INTERP_DELAY; default 2].
    #[arg(long, value_name = "INTERVALS", value_parser = parse_interp_delay)]
    pub net_interp_delay: Option<f32>,
    /// Show the network line of the metrics overlay at every level [env NFSMW_NET_SHOW_STATS; default off].
    #[arg(long, conflicts_with = "no_net_show_stats")]
    pub net_show_stats: bool,
    /// Show the network line at the advanced metrics level only.
    #[arg(long)]
    pub no_net_show_stats: bool,
    /// The identity key file [env NFSMW_NET_IDENTITY_FILE; default identity.key next to the config file].
    #[arg(long, value_name = "PATH", value_parser = |v: &str| IdentityPath::new(v))]
    pub net_identity_file: Option<IdentityPath>,
    /// The IP address a Discord invite carries for a game you host, for friends outside the LAN
    /// [env NFSMW_NET_PUBLIC_ADDRESS; default this PC's LAN address].
    #[arg(long, value_name = "IP", value_parser = parse_public_address)]
    pub net_public_address: Option<PublicAddress>,
}

impl NetArgs {
    pub fn layer(&self) -> NetLayer {
        let show_stats = match (self.net_show_stats, self.no_net_show_stats) {
            (true, _) => Some(true),
            (false, true) => Some(false),
            (false, false) => None,
        };
        NetLayer {
            name: self.net_name,
            port: self.net_port,
            snapshot_hz: self.net_snapshot_hz,
            interp_delay: self.net_interp_delay,
            show_stats,
            identity_file: self.net_identity_file,
            public_address: self.net_public_address,
        }
    }
}

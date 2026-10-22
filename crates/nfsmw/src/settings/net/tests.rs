use toml::Table;

use super::console::KEYS;
use super::io::*;
use super::*;

#[test]
fn the_defaults_are_the_servers_and_the_interpolators() {
    let s = NetSettings::default();
    assert_eq!(s.name.as_str(), "Player");
    assert_eq!((s.port, s.snapshot_hz, s.interp_delay, s.show_stats), (9750, 20, 2.0, false));
    assert!(s.identity_file.is_empty());
    let dir = std::path::Path::new("cfg");
    assert_eq!(s.identity_path(dir), dir.join("identity.key"));
}

#[test]
fn the_higher_layer_wins_and_values_are_clamped() {
    let high = NetLayer { port: Some(9800), interp_delay: Some(9.0), ..NetLayer::default() };
    let low = NetLayer { port: Some(9000), snapshot_hz: Some(30), show_stats: Some(true), ..NetLayer::default() };
    let s = NetSettings::from_layer(high.or(low));
    assert_eq!((s.port, s.snapshot_hz, s.interp_delay, s.show_stats), (9800, 30, 4.0, true));
    let odd = NetLayer { port: Some(80), snapshot_hz: Some(0), interp_delay: Some(f32::NAN), ..NetLayer::default() };
    let s = NetSettings::from_layer(odd);
    assert_eq!((s.port, s.snapshot_hz, s.interp_delay), (1024, 1, 2.0));
}

#[test]
fn the_config_keys_are_read_and_bad_values_left_unset() {
    let table: Table = "net_name = \"Pietro\"\nnet_port = 9760\nnet_snapshot_hz = 99\nnet_interp_delay = 1.5\n\
                        net_show_stats = true\nnet_identity_file = \"C:/keys/me.key\"\n"
        .parse()
        .unwrap();
    let layer = read(&table, "test");
    assert_eq!(layer.name.map(|n| n.to_string()), Some("Pietro".into()));
    assert_eq!(layer.port, Some(9760));
    assert_eq!(layer.snapshot_hz, None, "99 snapshots per second is refused");
    assert_eq!(layer.interp_delay, Some(1.5));
    assert_eq!(layer.show_stats, Some(true));
    assert_eq!(layer.identity_file.map(|p| p.to_string()), Some("C:/keys/me.key".into()));
    assert_eq!(read(&"vsync = true".parse().unwrap(), "test"), NetLayer::default());
}

#[test]
fn the_environment_is_read_and_bad_values_are_ignored() {
    let env = |name: &str| match name {
        NAME_ENV => Some("Ann".to_owned()),
        PORT_ENV => Some("80".to_owned()),
        SHOW_STATS_ENV => Some("on".to_owned()),
        _ => None,
    };
    let layer = from_env(env);
    assert_eq!(layer.name.map(|n| n.to_string()), Some("Ann".into()));
    assert_eq!(layer.port, None, "a system port is refused");
    assert_eq!(layer.show_stats, Some(true));
}

#[test]
fn written_values_read_back() {
    let layer = NetLayer::from(NetSettings { port: 9900, show_stats: true, ..NetSettings::default() });
    let mut table = Table::new();
    put_into(&layer, &mut |key, value| {
        table.insert(key.to_owned(), value);
    });
    assert_eq!(read(&table, "test"), layer);
}

#[test]
fn the_switches_and_options_map_to_layers() {
    let args = NetArgs { net_port: Some(9800), no_net_show_stats: true, ..NetArgs::default() };
    let layer = args.layer();
    assert_eq!((layer.port, layer.show_stats, layer.name), (Some(9800), Some(false), None));
}

#[test]
fn the_console_reads_sets_and_refuses() {
    let mut s = NetSettings::default();
    assert!(KEYS.iter().all(|key| is_key(key)) && !is_key("net_other"));
    set(&mut s, "net_name", "Marco").unwrap();
    set(&mut s, "net_port", "9760").unwrap();
    set(&mut s, "net_interp_delay", "3").unwrap();
    assert_eq!(get(&s, "net_name"), "Marco");
    assert_eq!(get(&s, "net_port"), "9760");
    assert_eq!(get(&s, "net_interp_delay"), "3");
    assert_eq!(get(&s, "net_identity_file"), "default");
    set(&mut s, "net_identity_file", "me.key").unwrap();
    assert_eq!(get(&s, "net_identity_file"), "me.key");
    set(&mut s, "net_identity_file", "default").unwrap();
    assert!(s.identity_file.is_empty());
    assert!(set(&mut s, "net_port", "22").is_err());
    assert!(set(&mut s, "net_snapshot_hz", "61").is_err());
    assert!(set(&mut s, "net_name", "").is_err());
    assert!(set(&mut s, "net_name", "a name far longer than twenty-four").is_err());
    *flag(&mut s, "net_show_stats").unwrap() = true;
    assert_eq!(get(&s, "net_show_stats"), "on");
    assert!(flag(&mut s, "net_port").is_none());
    assert_eq!(syntax("net_port"), Some("<1024-65534>"));
}

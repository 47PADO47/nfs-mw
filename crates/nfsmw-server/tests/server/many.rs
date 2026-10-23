//! Eight and sixteen clients at once, and the snapshot bandwidth against the 32 KB/s cap of features.md section 4.

use nfsmw_sim::toy::ToyConfig;

use super::harness::{Harness, test_config, throttle};

/// The hard cap on bytes per second to one client.
const CAP_BYTES_PER_S: f64 = 32.0 * 1024.0;

/// Joins `count` clients, starts a free roam, drives `seconds` of full throttle and returns each client's snapshot
/// bytes per second over that time.
fn session(count: usize, seconds: u32, toy: ToyConfig) -> Vec<f64> {
    let mut config = test_config();
    nfsmw_net::settings::set(&mut config.settings, "max_players", &count.max(2).to_string()).expect("valid");
    let mut h = Harness::new(config, toy);
    let clients: Vec<usize> = (0..count).map(|index| h.connect(100 + index as u8)).collect();
    h.join_all(&clients);
    h.start_race(&clients);
    let before: Vec<u64> = clients.iter().map(|&index| h.clients[index].snapshot_bytes).collect();
    let counts: Vec<usize> = clients.iter().map(|&index| h.clients[index].snapshots.len()).collect();
    h.run(seconds * 60, throttle());
    let mut rates = Vec::new();
    for (position, &index) in clients.iter().enumerate() {
        let client = &h.clients[index];
        assert_eq!(client.snapshot_errors, 0, "client {index}: every snapshot decodes");
        let got = client.snapshots.len() - counts[position];
        assert!(got as u32 >= seconds * 18, "client {index}: {got} snapshots in {seconds} s");
        let last = client.snapshots.last().expect("snapshots");
        assert!(last.own.is_some(), "client {index} has its own block");
        let view = client.views.latest().expect("a view");
        assert!(view.len() >= count, "client {index} sees every player car: {}", view.len());
        rates.push((client.snapshot_bytes - before[position]) as f64 / f64::from(seconds));
    }
    rates
}

#[test]
fn eight_clients_stay_under_the_bandwidth_cap() {
    let rates = session(8, 5, ToyConfig::default());
    let max = rates.iter().copied().fold(0.0, f64::max);
    let mean = rates.iter().sum::<f64>() / rates.len() as f64;
    println!(
        "8 players, 8 traffic, 2 police, 20 Hz: snapshot bytes per client per second: mean {mean:.0}, max {max:.0}"
    );
    assert!(max < CAP_BYTES_PER_S, "max {max:.0} B/s is over the cap");
}

#[test]
fn eight_clients_with_heavy_traffic_stay_under_the_bandwidth_cap() {
    let toy = ToyConfig { traffic: 120, police: 8, ..ToyConfig::default() };
    let rates = session(8, 4, toy);
    let max = rates.iter().copied().fold(0.0, f64::max);
    let mean = rates.iter().sum::<f64>() / rates.len() as f64;
    println!(
        "8 players, 120 traffic, 8 police, 20 Hz: snapshot bytes per client per second: mean {mean:.0}, max {max:.0}"
    );
    assert!(max < CAP_BYTES_PER_S, "max {max:.0} B/s is over the cap");
}

#[test]
fn sixteen_clients_play_at_once() {
    let rates = session(16, 3, ToyConfig::default());
    let max = rates.iter().copied().fold(0.0, f64::max);
    println!("16 players, 20 Hz: snapshot bytes per client per second: max {max:.0}");
    assert!(max < CAP_BYTES_PER_S);
}

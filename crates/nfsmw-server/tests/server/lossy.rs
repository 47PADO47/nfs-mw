//! Clients that run the real Noise handshake over the seeded lossy simulator: snapshots keep decoding because the
//! server only takes a snapshot as a baseline once the packet that carried it was acknowledged.

use nfsmw_sim::toy::ToyConfig;

use super::harness::{Harness, test_config, throttle};

#[test]
fn snapshots_decode_under_two_percent_loss_with_acknowledged_baselines() {
    let mut h = Harness::lossy(test_config(), ToyConfig::default(), 0.02);
    let clients: Vec<usize> = (0..3).map(|seed| h.connect_lossy(80 + seed, 0.02)).collect();
    h.join_all(&clients);
    h.start_race(&clients);
    h.run(300, throttle());
    for &index in &clients {
        let client = &h.clients[index];
        assert_eq!(client.snapshot_errors, 0, "client {index}: a delta never names a baseline the client lacks");
        let deltas = client.snapshots.iter().filter(|snapshot| snapshot.baseline_distance > 0).count();
        assert!(deltas >= 60, "client {index}: {deltas} deltas");
        assert!(client.acked.is_some(), "client {index}: inputs acknowledged");
    }
    let stats = h.server.stats();
    assert!(stats.snapshots_delivered > 0);
    println!(
        "lossy: {} snapshots sent, {} delivered, {} lost",
        stats.snapshots_sent, stats.snapshots_delivered, stats.snapshots_lost
    );
}

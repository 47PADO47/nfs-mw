use super::*;

#[test]
fn three_numbers_parse() {
    let sim = NetSim::parse("2 120 10").unwrap();
    assert_eq!((sim.loss_percent(), sim.rtt_ms(), sim.jitter_ms()), (2.0, 120, 10));
    assert_eq!(NetSim::parse(" 0.5%  40ms 5ms ").unwrap(), NetSim::new(0.5, 40, 5).unwrap());
    assert_eq!(sim.to_string(), "net.sim 2 120 10");
}

#[test]
fn off_turns_it_off() {
    let off = NetSim::parse("off").unwrap();
    assert!(off.is_off());
    assert_eq!(off.to_string(), "net.sim off");
    assert!(NetSim::parse("0 0 0").unwrap().is_off());
}

#[test]
fn bad_settings_are_refused() {
    assert_eq!(NetSim::parse(""), Err(NetSimError::Usage));
    assert_eq!(NetSim::parse("1 2"), Err(NetSimError::Usage));
    assert_eq!(NetSim::parse("1 2 3 4"), Err(NetSimError::Usage));
    assert_eq!(NetSim::parse("x 2 3"), Err(NetSimError::NotANumber("x".into())));
    assert_eq!(NetSim::parse("1 -2 3"), Err(NetSimError::NotANumber("-2".into())));
    assert_eq!(NetSim::parse("60 2 3"), Err(NetSimError::Loss));
    assert_eq!(NetSim::parse("-1 2 3"), Err(NetSimError::Loss));
    assert_eq!(NetSim::parse("NaN 2 3"), Err(NetSimError::Loss));
    assert_eq!(NetSim::parse("1 5000 3"), Err(NetSimError::Rtt));
    assert_eq!(NetSim::parse("1 2 3000"), Err(NetSimError::Jitter));
}

#[test]
fn the_round_trip_and_jitter_are_split_between_the_directions() {
    let pair = NetSim::new(5.0, 151, 21).unwrap().link_configs(7);
    assert_eq!((pair.send.latency_ms, pair.receive.latency_ms), (76, 75));
    assert_eq!((pair.send.jitter_ms, pair.receive.jitter_ms), (11, 10));
    assert!((pair.send.loss - 0.05).abs() < 1.0e-9);
    assert_eq!(pair.send.loss, pair.receive.loss);
    assert_eq!(pair.send.seed, 7);
    assert_ne!(pair.receive.seed, pair.send.seed);
    assert_eq!((pair.send.duplicate, pair.send.reorder), (0.0, 0.0));
}

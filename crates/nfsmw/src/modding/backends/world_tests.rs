//! `bb.world`, `bb.hud` and `bb.audio` on the game side: the track and the district come from the sync, the rain and the traffic
//! are queued for the scene, a HUD message is queued for the card, and the audio table answers unsupported.

use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_NOT_FOUND, BB_ERR_NOT_READY, BB_ERR_UNSUPPORTED};
use blackbox_modding::host::{AudioBackend, HudBackend, MessageStyle, WorldBackend};

use super::audio::AudioPort;
use super::bridge::Bridge;
use super::hud::HudPort;
use super::support::Recorder;
use super::world::{WorldPort, WorldRequest, apply};
use crate::hud::{ModMessageTone, current_mod_message};
use crate::viewer::WorldInfo;

fn port() -> (Rc<Bridge>, WorldPort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), WorldPort(bridge))
}

#[test]
fn the_track_is_unavailable_without_a_world() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::without_clock());
    assert_eq!(port.track("example.a"), Err(BB_ERR_NOT_READY));
    assert_eq!(port.set_rain("example.a", true, 0.5), Err(BB_ERR_NOT_READY));
    assert!(bridge.world_requests.borrow().is_empty());
}

#[test]
fn the_track_and_the_section_come_from_the_sync() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::driving([0.0; 3], 0.0));
    assert_eq!(port.track("example.a").as_deref(), Ok("Bayview"));
    assert_eq!(port.section("example.a").as_deref(), Ok("Harbor"));
}

#[test]
fn a_section_the_install_does_not_name_is_not_found() {
    let (bridge, port) = port();
    let mut scene = Recorder::driving([0.0; 3], 0.0);
    scene.track = Some(WorldInfo { track: "Bayview".to_owned(), section: None });
    bridge.sync(&mut scene);
    assert_eq!(port.section("example.a"), Err(BB_ERR_NOT_FOUND));
}

#[test]
fn rain_and_traffic_are_queued_and_reach_the_scene() {
    let (bridge, port) = port();
    bridge.sync(&mut Recorder::driving([0.0; 3], 0.0));
    port.set_rain("example.a", true, 0.5).expect("a world is loaded");
    port.set_traffic("example.a", 30, 25).expect("a world is loaded");
    assert_eq!(port.set_traffic("example.a", 30, 300), Err(BB_ERR_ARG), "a cop share past a byte is refused");
    let requests = std::mem::take(&mut *bridge.world_requests.borrow_mut());
    let mut scene = Recorder::driving([0.0; 3], 0.0);
    apply(requests, &mut scene);
    assert_eq!(scene.rain, Some((true, 0.5)));
    assert_eq!(scene.traffic, Some((30, 25)));
}

#[test]
fn a_hud_message_is_queued_and_shown_until_it_is_hidden() {
    let bridge = Rc::new(Bridge::default());
    let hud = HudPort(bridge.clone());
    bridge.sync(&mut Recorder::driving([0.0; 3], 0.0));
    hud.message("example.a", "Pursuit", MessageStyle::Big, 4.0).expect("the message is queued");
    let queued = std::mem::take(&mut *bridge.world_requests.borrow_mut());
    let expected = WorldRequest::Message { text: "Pursuit".to_owned(), tone: ModMessageTone::Big, secs: 4.0 };
    assert_eq!(queued, vec![expected]);
    apply(queued, &mut Recorder::driving([0.0; 3], 0.0));
    let shown = current_mod_message().expect("the card is up");
    assert_eq!((shown.text.as_str(), shown.tone), ("Pursuit", ModMessageTone::Big));

    hud.hide("example.a").expect("the hide is queued");
    let hide = std::mem::take(&mut *bridge.world_requests.borrow_mut());
    apply(hide, &mut Recorder::driving([0.0; 3], 0.0));
    assert!(current_mod_message().is_none());
}

#[test]
fn the_audio_table_answers_unsupported_until_event_names_are_looked_up() {
    assert_eq!(AudioPort.play("example.a", "IG_GLOBAL", "Siren", 0.5), Err(BB_ERR_UNSUPPORTED));
}

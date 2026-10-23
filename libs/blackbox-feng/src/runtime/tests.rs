use glam::Quat;

use super::*;
use crate::hash::fe_hash_upper;
use crate::package::synth::*;
use crate::package::word;
use crate::runtime::state::START_HIDDEN;
use crate::tree::UiTree;

const FADEIN: u32 = 0x5b0d9106;
const HIDE: u32 = 0x0016a259;
const INIT: u32 = INIT_SCRIPT;

fn colour_track(keys: Vec<(i32, [i32; 4])>, length: u32) -> TrackSpec {
    TrackSpec {
        param: 6,
        interp: 1,
        action: 0,
        offset: 0,
        length,
        keys: keys.into_iter().map(|(t, v)| (t, v.iter().map(|x| *x as u32).collect())).collect(),
    }
}

fn fade_script() -> ScriptSpec {
    ScriptSpec {
        id: FADEIN,
        length: 600,
        flags: 0,
        chain: None,
        tracks: vec![colour_track(vec![(-1, [255; 4]), (0, [0, 0, 0, -255]), (600, [0; 4])], 600)],
        events: vec![],
    }
}

fn hidden_init() -> ScriptSpec {
    ScriptSpec {
        id: INIT,
        length: 0,
        flags: 0,
        chain: None,
        tracks: vec![colour_track(vec![(-1, [255, 255, 255, 0])], 0)],
        events: vec![],
    }
}

/// A package with one image that starts transparent, fades in on message 0x10 and tells the game at its end.
fn fading_image() -> Vec<u8> {
    let mut image = Obj::image(1, fe_hash_upper("Panel"));
    image.colour = [255, 255, 255, 255];
    image.scripts.push(hidden_init().build());
    let mut fade = fade_script();
    fade.events.push((0x99, 0xFFFF_FFFF, 600));
    image.scripts.push(fade.build());
    image.responses = responses(&[(0x10, vec![(0, FADEIN, 0)])]);
    package("Fade.fng", &[], &[image], &[], &[(0x10, vec![1])])
}

fn load(bytes: &[u8]) -> (Runtime, PackageId) {
    let mut rt = Runtime::new();
    let id = rt.load(Package::parse(bytes).unwrap());
    (rt, id)
}

fn alpha(rt: &Runtime, o: ObjectRef) -> i32 {
    rt.object(o).unwrap().data.alpha()
}

#[test]
fn init_scripts_apply_before_the_first_tree_is_presented() {
    let (mut rt, id) = load(&fading_image());
    let panel = rt.find(id, fe_hash_upper("PANEL")).unwrap();
    assert_eq!(alpha(&rt, panel), 0, "INIT must hide the stored end pose before drawing");
    assert_eq!(rt.tree(id).nodes[panel.index].world_colour[3], 0);
    rt.update(1.0 / 60.0);
    assert_eq!(alpha(&rt, panel), 0, "INIT makes it transparent");
    assert_eq!(rt.script_of(panel), Some(INIT));
}

#[test]
fn loading_primes_the_entrance_without_advancing_clocks_or_repeating_init_events() {
    let mut obj = Obj::image(1, fe_hash_upper("Panel"));
    let mut init = hidden_init();
    init.chain = Some(FADEIN);
    init.events.push((0x88, 0xFFFF_FFFF, 0));
    obj.scripts.extend([init.build(), fade_script().build()]);
    let bytes = package("Entrance.fng", &[], &[obj], &[], &[]);
    let (mut rt, first) = load(&bytes);
    let first_panel = rt.find_guid(first, 1).unwrap();
    assert_eq!(rt.script_of(first_panel), Some(FADEIN));
    assert_eq!(rt.object(first_panel).unwrap().time, 0);
    assert_eq!(alpha(&rt, first_panel), 0);
    assert!(rt.take_outgoing().is_empty(), "initialization messages wait for the next update");

    rt.update(0.3125);
    assert_eq!(rt.take_outgoing(), vec![Outgoing::Game { message: 0x88, package: first, from: Some(1) }]);
    let first_state = rt.object(first_panel).unwrap().clone();
    let remainder = rt.remainder;
    let second = rt.load(Package::parse(&bytes).unwrap());
    let second_panel = rt.find_guid(second, 1).unwrap();
    assert_eq!(rt.object(second_panel).unwrap().time, 0);
    assert_eq!(alpha(&rt, second_panel), 0);
    assert_eq!(rt.object(first_panel).unwrap().time, first_state.time);
    assert_eq!(rt.object(first_panel).unwrap().data.words, first_state.data.words);
    assert_eq!(rt.remainder, remainder);

    rt.update(1.0 / 60.0);
    assert_eq!(rt.take_outgoing(), vec![Outgoing::Game { message: 0x88, package: second, from: Some(1) }]);
    assert_eq!(rt.object(second_panel).unwrap().time, 16);
    assert!((1..255).contains(&alpha(&rt, second_panel)));
    rt.update(1.0 / 60.0);
    assert!(rt.take_outgoing().is_empty(), "each INIT event fires once");
}

#[test]
fn a_message_starts_a_fade_that_takes_600_ticks() {
    let (mut rt, id) = load(&fading_image());
    let panel = rt.find(id, fe_hash_upper("PANEL")).unwrap();
    rt.update(1.0 / 60.0);
    rt.post(id, 0x10, None);
    rt.update(1.0 / 60.0);
    assert_eq!(rt.script_of(panel), Some(FADEIN));
    // Half way: 300 ticks = 0.3125 s.
    rt.update(0.3125);
    let a = alpha(&rt, panel);
    assert!((100..160).contains(&a), "alpha {a}");
    rt.update(0.5);
    assert_eq!(alpha(&rt, panel), 255);
    // The event at tick 600 told the game.
    let out = rt.take_outgoing();
    assert_eq!(out, vec![Outgoing::Game { message: 0x99, package: id, from: Some(1) }]);
    assert!(rt.take_outgoing().is_empty());
}

#[test]
fn configured_tick_rate_preserves_fade_timing_across_render_rates() {
    for fps in [5, 60, 144, 360] {
        let mut rt = Runtime::with_tick_rate(1200.0);
        let id = rt.load(Package::parse(&fading_image()).unwrap());
        let panel = rt.find(id, fe_hash_upper("PANEL")).unwrap();
        rt.post(id, 0x10, None);
        rt.update(1.0 / 1200.0);
        for _ in 0..fps {
            rt.update(0.25 / fps as f32);
        }
        assert!((100..160).contains(&alpha(&rt, panel)), "half fade at {fps}");
        for _ in 0..fps {
            rt.update(0.25 / fps as f32);
        }
        assert_eq!(alpha(&rt, panel), 255, "complete fade at {fps}");
        // f32 frame deltas can land just below the final authored tick.
        // The completion event must arrive by the following single tick.
        rt.update(1.0 / 1200.0);
        assert!(rt.take_outgoing().iter().any(|event| matches!(event, Outgoing::Game { message: 0x99, .. })));
    }
}

#[test]
fn values_the_host_sets_survive_a_script_at_rest() {
    let (mut rt, id) = load(&fading_image());
    let panel = rt.find(id, fe_hash_upper("PANEL")).unwrap();
    for _ in 0..3 {
        rt.update(1.0 / 60.0);
    }
    rt.set_rotation_z(panel, 1.0);
    rt.set_alpha(panel, 200);
    for _ in 0..30 {
        rt.update(1.0 / 60.0);
    }
    let s = rt.object(panel).unwrap();
    assert_eq!(s.data.alpha(), 200);
    assert!((s.data.rotation().to_axis_angle().1 - 1.0).abs() < 1e-5);
}

#[test]
fn tracks_other_than_colour_wait_while_the_object_is_transparent() {
    let mut obj = Obj::image(1, fe_hash_upper("Mover"));
    obj.colour = [255, 255, 255, 0];
    let mut s = ScriptSpec {
        id: INIT,
        length: 100,
        flags: 0,
        chain: None,
        tracks: vec![TrackSpec {
            param: 4,
            interp: 1,
            action: 0,
            offset: word::POSITION as u32,
            length: 100,
            keys: vec![
                (-1, f32s_to_words(&[0.0, 0.0, 10.0])),
                (0, f32s_to_words(&[0.0, 0.0, 0.0])),
                (100, f32s_to_words(&[50.0, 0.0, 0.0])),
            ],
        }],
        events: vec![],
    };
    obj.scripts.push(s.build());
    let (mut rt, id) = load(&package("Mover.fng", &[], &[obj], &[], &[]));
    let o = rt.find(id, fe_hash_upper("MOVER")).unwrap();
    for _ in 0..30 {
        rt.update(1.0 / 60.0);
    }
    assert_eq!(rt.object(o).unwrap().data.position().x, 0.0, "transparent: no motion");
    rt.set_alpha(o, 255);
    s.id = INIT;
    rt.run_script(o, INIT);
    rt.update(0.05);
    assert!(rt.object(o).unwrap().data.position().x > 0.0, "visible: moves");
}

fn f32s_to_words(v: &[f32]) -> Vec<u32> {
    v.iter().map(|f| f.to_bits()).collect()
}

#[test]
fn loops_chains_and_ping_pong() {
    let mut obj = Obj::image(1, fe_hash_upper("Blinker"));
    // INIT (length 0) chains to a looping script of 100 ticks that sends an event at tick 50.
    let mut init = hidden_init();
    init.chain = Some(0xA1);
    obj.scripts.push(init.build());
    let looping = ScriptSpec {
        id: 0xA1,
        length: 100,
        flags: 1,
        chain: None,
        tracks: vec![],
        events: vec![(0x77, 0xFFFF_FFFF, 50)],
    };
    obj.scripts.push(looping.build());
    let (mut rt, id) = load(&package("Loop.fng", &[], &[obj], &[], &[]));
    // 20 frames of 16 ticks = 320 ticks: events at 50, 150, 250.
    for _ in 0..20 {
        rt.update(1.0 / 60.0);
    }
    let n = rt.take_outgoing().iter().filter(|o| matches!(o, Outgoing::Game { message: 0x77, .. })).count();
    assert_eq!(n, 3);
    let o = rt.find(id, fe_hash_upper("BLINKER")).unwrap();
    assert_eq!(rt.script_of(o), Some(0xA1));
}

#[test]
fn focus_sends_gain_and_lose_messages() {
    let mut a = Obj::image(1, fe_hash_upper("A"));
    a.scripts.push(hidden_init().build());
    a.responses = responses(&[(0xabc08912, vec![(0, FADEIN, 0)]), (0x55d1e635, vec![(0, HIDE, 0)])]);
    a.scripts.push(fade_script().build());
    let mut hide = fade_script();
    hide.id = HIDE;
    a.scripts.push(hide.build());
    let mut b = Obj::image(2, fe_hash_upper("B"));
    b.scripts.push(hidden_init().build());
    b.responses = responses(&[(0xabc08912, vec![(2, 0x55, 0xFFFF_FFFF)])]);
    let (mut rt, id) = load(&package("Btn.fng", &[], &[a, b], &[], &[]));
    rt.update(1.0 / 60.0);
    rt.set_focus(id, 1);
    rt.update(1.0 / 60.0);
    let oa = rt.find_guid(id, 1).unwrap();
    assert_eq!(rt.script_of(oa), Some(FADEIN));
    assert_eq!(rt.focus(id), Some(1));
    rt.set_focus(id, 2);
    rt.update(1.0 / 60.0);
    assert_eq!(rt.script_of(oa), Some(HIDE), "the old button got the lose message");
    assert!(rt.take_outgoing().contains(&Outgoing::Game { message: 0x55, package: id, from: Some(2) }));
    rt.set_focus(id, 99);
    assert_eq!(rt.focus(id), Some(2), "an unknown GUID changes nothing");
}

#[test]
fn if_script_equals_picks_a_branch() {
    // On message 0x20: if the object plays INIT, post 0x31 to the game, else post 0x32.
    let mut o = Obj::image(1, fe_hash_upper("Cond"));
    o.scripts.push(hidden_init().build());
    o.scripts.push(fade_script().build());
    // 0x300 if equals, 2 post to game, 0x500 else, 2 post, 0x501 end if, then an unconditional response.
    o.responses = responses(&[
        (
            0x20,
            vec![
                (0x300, INIT, 0),
                (2, 0x31, 0xFFFF_FFFF),
                (0x500, 0, 0),
                (2, 0x32, 0xFFFF_FFFF),
                (0x501, 0, 0),
                (2, 0x33, 0xFFFF_FFFF),
            ],
        ),
        (0x21, vec![(0, FADEIN, 0)]),
    ]);
    let (mut rt, id) = load(&package("Cond.fng", &[], &[o], &[], &[(0x20, vec![1]), (0x21, vec![1])]));
    rt.update(1.0 / 60.0);
    rt.post(id, 0x20, None);
    rt.update(1.0 / 60.0);
    let messages = |out: Vec<Outgoing>| -> Vec<u32> {
        out.into_iter()
            .filter_map(|o| if let Outgoing::Game { message, .. } = o { Some(message) } else { None })
            .collect()
    };
    assert_eq!(messages(rt.take_outgoing()), vec![0x31, 0x33]);
    rt.post(id, 0x21, None);
    rt.update(1.0 / 60.0);
    rt.post(id, 0x20, None);
    rt.update(1.0 / 60.0);
    assert_eq!(messages(rt.take_outgoing()), vec![0x32, 0x33]);
}

#[test]
fn package_responses_and_commands_reach_the_host() {
    let pkg_resp = responses(&[(0x40, vec![(1, 0x41, 0xFFFF_FFFC), (0x200, 0, 0)])]);
    let (mut rt, id) = load(&package("Pkg.fng", &[], &[], &pkg_resp, &[]));
    rt.post(id, 0x40, None);
    rt.update(1.0 / 60.0);
    // The package-control response has a numeric parameter here, so the name is empty.
    let out = rt.take_outgoing();
    assert_eq!(out, vec![Outgoing::Package { kind: PackageCommandKind::Switch, name: String::new(), from: id }]);
}

#[test]
fn text_binding_and_label_resolution() {
    let mut text = Obj::string(1, fe_hash_upper("Units"), "KM/H");
    text.label = 0x8569a25f;
    let (mut rt, id) = load(&package("Text.fng", &[], &[text], &[], &[]));
    rt.set_string_resolver(|label| (label == 0x8569a25f).then(|| "MPH".to_string()));
    let o = rt.find(id, fe_hash_upper("units")).unwrap();
    rt.update(1.0 / 60.0);
    assert_eq!(rt.tree(id).nodes[0].text.as_deref(), Some("MPH"));
    rt.set_text(o, "42");
    assert_eq!(rt.tree(id).nodes[0].text.as_deref(), Some("42"));
}

#[test]
fn rotation_binding_composes_into_the_tree() {
    let mut img = Obj::image(1, fe_hash_upper("Needle"));
    img.position = [10.0, 20.0, 5.0];
    let (mut rt, id) = load(&package("Needle.fng", &[], &[img], &[], &[]));
    let o = rt.find(id, fe_hash_upper("needle")).unwrap();
    rt.update(1.0 / 60.0);
    rt.set_rotation_z(o, std::f32::consts::FRAC_PI_2);
    let tree = rt.tree(id);
    let n = &tree.nodes[0];
    let q = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    assert!((n.local_rotation.dot(q).abs() - 1.0).abs() < 1e-5);
}

#[test]
fn start_hidden_object_shows_once_a_script_is_set_on_it() {
    // Definition flag 8 hides the object at load. A response that sets its script reveals it. A script the
    // object does not own reveals nothing.
    const SHOW: u32 = 0x5a0c_1f2e;
    const OTHER: u32 = 0x5a0c_1f2f;
    let mut panel = Obj::image(1, fe_hash_upper("Panel"));
    panel.position = [0.0, 0.0, 1.0];
    panel.flags = START_HIDDEN;
    let show = ScriptSpec { id: SHOW, length: 0, flags: 0, chain: None, tracks: vec![], events: vec![] };
    panel.scripts.push(show.build());
    panel.responses = responses(&[(0x30, vec![(0, SHOW, 0)]), (0x31, vec![(0, OTHER, 0)])]);
    let (mut rt, id) = load(&package("Start.fng", &[], &[panel], &[], &[(0x30, vec![1]), (0x31, vec![1])]));
    let o = rt.find(id, fe_hash_upper("Panel")).unwrap();
    rt.update(1.0 / 60.0);
    assert!(rt.object(o).unwrap().hidden);
    assert!(rt.tree(id).draw_order.is_empty(), "a start-hidden object does not draw");

    rt.post(id, 0x31, None);
    rt.update(1.0 / 60.0);
    assert!(rt.object(o).unwrap().hidden, "a script the object does not own keeps it hidden");

    rt.post(id, 0x30, None);
    rt.update(1.0 / 60.0);
    assert_eq!(rt.script_of(o), Some(SHOW));
    assert!(!rt.object(o).unwrap().hidden);
    assert_eq!(rt.tree(id).draw_order, vec![0]);
}

fn digits_package() -> (Runtime, PackageId, ObjectRef) {
    let mut digits = Obj::string(1, fe_hash_upper("Digits"), "0");
    digits.position = [0.0, 0.0, 1.0];
    let mut panel = Obj::image(2, fe_hash_upper("Panel"));
    panel.position = [0.0, 0.0, 2.0];
    let (mut rt, id) = load(&package("Rev.fng", &[], &[digits, panel], &[], &[]));
    rt.update(1.0 / 60.0);
    let o = rt.find(id, fe_hash_upper("digits")).unwrap();
    (rt, id, o)
}

#[test]
fn an_unknown_package_has_revision_zero_and_a_loaded_one_does_not() {
    let (rt, id, _) = digits_package();
    assert_ne!(rt.revision(id), 0);
    assert_eq!(rt.revision(PackageId(999)), 0);
}

#[test]
fn set_text_with_the_text_already_shown_leaves_the_revision_alone() {
    let (mut rt, id, o) = digits_package();
    rt.set_text(o, "42");
    let after_first = rt.revision(id);
    rt.set_text(o, "42");
    rt.set_text(o, String::from("42"));
    rt.set_text_str(o, "42");
    assert_eq!(rt.revision(id), after_first, "an equal text is a no-op");
    rt.set_text_str(o, "43");
    assert_ne!(rt.revision(id), after_first, "another text moves it");
    assert_eq!(rt.tree(id).nodes[0].text.as_deref(), Some("43"));
    let after_change = rt.revision(id);
    rt.set_text(o, "44");
    assert_ne!(rt.revision(id), after_change);
    assert_eq!(rt.tree(id).nodes[0].text.as_deref(), Some("44"));
}

#[test]
fn setters_that_repeat_a_value_leave_the_revision_alone() {
    let (mut rt, id, o) = digits_package();
    let panel = rt.find(id, fe_hash_upper("panel")).unwrap();
    rt.set_alpha(panel, 200);
    rt.set_colour_rgba(panel, [10, 20, 30, 200]);
    rt.set_rotation_z(panel, 0.5);
    rt.set_position_xy(panel, 4.0, 5.0);
    rt.set_hidden(panel, true);
    rt.set_texture(panel, 7);
    rt.set_label(o, 0);
    rt.set_text(o, "1");
    let settled = rt.revision(id);
    rt.set_alpha(panel, 200);
    rt.set_colour_rgba(panel, [10, 20, 30, 200]);
    rt.set_rotation_z(panel, 0.5);
    rt.set_position_xy(panel, 4.0, 5.0);
    rt.set_hidden(panel, true);
    rt.set_texture(panel, 7);
    rt.set_text(o, "1");
    assert_eq!(rt.revision(id), settled);
    rt.set_hidden(panel, false);
    assert_ne!(rt.revision(id), settled, "showing an object changes the tree");
}

#[test]
fn updating_a_package_with_nothing_playing_keeps_the_revision() {
    let (mut rt, id, _) = digits_package();
    let settled = rt.revision(id);
    rt.update(1.0 / 60.0);
    rt.update(0.5);
    assert_eq!(rt.revision(id), settled);
}

#[test]
fn a_playing_script_and_a_new_resolver_move_the_revision() {
    let (mut rt, id) = load(&fading_image());
    rt.update(1.0 / 60.0);
    let settled = rt.revision(id);
    rt.update(1.0 / 60.0);
    assert_eq!(rt.revision(id), settled, "the entrance pose is at rest");
    rt.post(id, 0x10, None);
    rt.update(1.0 / 60.0);
    let started = rt.revision(id);
    assert_ne!(started, settled, "the fade script is set");
    rt.update(0.1);
    assert_ne!(rt.revision(id), started, "the fade moves the alpha");
    let moving = rt.revision(id);
    rt.set_string_resolver(|_| None);
    assert_ne!(rt.revision(id), moving, "every label may read differently");
}

#[test]
fn tree_into_reuses_a_buffer_and_matches_tree() {
    let (mut rt, id, o) = digits_package();
    let mut buffer = UiTree::default();
    rt.tree_into(id, &mut buffer);
    assert_eq!(format!("{buffer:?}"), format!("{:?}", rt.tree(id)));
    rt.set_text(o, "a longer text than before");
    rt.tree_into(id, &mut buffer);
    assert_eq!(format!("{buffer:?}"), format!("{:?}", rt.tree(id)));
    rt.set_text(o, "x");
    rt.tree_into(id, &mut buffer);
    assert_eq!(buffer.nodes[0].text.as_deref(), Some("x"));
    assert_eq!(format!("{buffer:?}"), format!("{:?}", rt.tree(id)));
    rt.unload(id);
    rt.tree_into(id, &mut buffer);
    assert!(buffer.nodes.is_empty() && buffer.draw_order.is_empty());
}

use bevy_app::{App, TaskPoolPlugin};
use bevy_asset::{Asset, AssetApp, AssetEvent, AssetPlugin};
use bevy_ecs::message::Messages;
use bevy_ecs::system::SystemState;
use bevy_math::Vec4;

use super::*;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((TaskPoolPlugin::default(), AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<BlackboxMaterial>()
        .init_asset::<ShaderBuffer>();
    app
}

fn with_stores<R>(app: &mut App, f: impl FnOnce(&mut Stores) -> R) -> R {
    let mut system_state = SystemState::<Stores>::new(app.world_mut());
    let mut stores = system_state.get_mut(app.world_mut()).unwrap();
    f(&mut stores)
}

/// The `Modified` events of asset type `A` since the last call.
fn modified_events<A: Asset>(app: &mut App) -> usize {
    let mut events = app.world_mut().resource_mut::<Messages<AssetEvent<A>>>();
    events.drain().filter(|event| matches!(event, AssetEvent::Modified { .. })).count()
}

/// How many materials and scene buffers were modified since the last call.
fn modified(app: &mut App) -> (usize, usize) {
    app.update();
    (modified_events::<BlackboxMaterial>(app), modified_events::<ShaderBuffer>(app))
}

#[test]
fn a_changing_scene_rewrites_the_buffer_and_touches_no_material() {
    let mut app = app();
    let mut state = WorldState::default();
    with_stores(&mut app, |stores| {
        sync_scene(&mut state, stores, Params::default());
        material_for(&mut state, stores, None, BlendKind::Opaque, ShadingKind::Lit, None);
        material_for(&mut state, stores, None, BlendKind::Opaque, ShadingKind::Glossy, Some(1));
    });
    modified(&mut app);
    for step in 0..5 {
        let wanted = Params { fog_color: Vec4::splat(step as f32 / 10.0), ..Params::default() };
        with_stores(&mut app, |stores| sync_scene(&mut state, stores, wanted));
        assert_eq!(modified(&mut app), (0, 1), "frame {step}: one buffer write, no material");
    }
}

#[test]
fn an_unchanged_scene_writes_nothing() {
    let mut app = app();
    let mut state = WorldState::default();
    with_stores(&mut app, |stores| sync_scene(&mut state, stores, Params::default()));
    modified(&mut app);
    with_stores(&mut app, |stores| sync_scene(&mut state, stores, Params::default()));
    assert_eq!(modified(&mut app), (0, 0));
}

#[test]
fn a_new_rig_reaches_the_buffer_on_the_next_sync() {
    let mut app = app();
    let mut state = WorldState::default();
    with_stores(&mut app, |stores| sync_scene(&mut state, stores, Params::default()));
    state.rig.ambient = Vec4::new(0.2, 0.3, 0.4, 0.0);
    with_stores(&mut app, |stores| sync_scene(&mut state, stores, Params::default()));
    assert_eq!(modified(&mut app).1, 1 + 1, "added once, then modified by the rig change");
}

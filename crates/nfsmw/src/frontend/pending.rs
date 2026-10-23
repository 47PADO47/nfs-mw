//! The front end while its screens load on a background thread. The window opens at once and shows the scene alone
//! until the screens and the UI assets are read; the front end then starts exactly as it did when they were read before
//! the window opened. Its systems wait for it (`run_if(resource_exists::<Frontend>)` in `plugin.rs`), and a screenshot
//! or a benchmark waits for the load (`UiLoading`), so a scripted run's first step comes after it.

use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use game_install::GameDir;

use super::browse::Browse;
use super::camera_rig::CameraRig;
use super::flow::{Frontend, Start};
use super::look::Look;
use super::plugin::FrontendPlugin;
use super::screens::Screens;
use super::script::UiScript;
use crate::app::{FrameSet, Host};
use crate::career::CareerHandle;
use crate::ui::{AssetsLoad, Job, UiLoading};

/// The name of the front end's load in [`UiLoading`].
const PART: &str = "the front end";

/// What the front end starts with besides its screens: the plugin's fields.
pub(super) struct Setup {
    pub dir: GameDir,
    pub start: Start,
    pub script: Option<UiScript>,
    pub browse: Option<Browse>,
    pub career: CareerHandle,
    pub look: Option<Look>,
    pub camera: CameraRig,
}

impl Setup {
    pub fn of(plugin: &FrontendPlugin) -> Self {
        Self {
            dir: plugin.dir.clone(),
            start: plugin.start.clone(),
            script: plugin.script.clone(),
            browse: plugin.browse.clone(),
            career: plugin.career.clone(),
            look: plugin.look.clone(),
            camera: plugin.camera.clone(),
        }
    }

    /// The front end over `screens`, as the plugin always built it.
    pub fn build(self, screens: Screens) -> Frontend {
        let mut frontend = Frontend::new(screens, self.dir, self.start, self.script);
        frontend.browse = self.browse;
        frontend.career = self.career;
        frontend.camera = self.camera;
        let Some(look) = self.look else { return frontend };
        frontend.look = look.clone();
        frontend.look_override = Some(look);
        frontend
    }
}

/// The front end being built on its loader thread; `None` from the thread when the UI assets could not be read.
#[derive(Resource)]
pub struct PendingFrontend(Job<Option<Frontend>>);

impl PendingFrontend {
    /// Reads the screens' packages beside the UI assets (they share files through the session memo, so the second
    /// reader of a file waits for the first) and builds the front end over them.
    pub(super) fn start(setup: Setup, assets: AssetsLoad) -> Self {
        Self(Job::spawn("frontend-load", move || {
            let screens = Screens::load(&setup.dir, &assets)?;
            Some(setup.build(screens))
        }))
    }

    /// A front end that is already built (tests).
    #[cfg(test)]
    pub(super) fn ready(frontend: Option<Frontend>) -> Self {
        Self(Job::spawn("frontend-test", move || frontend))
    }
}

/// Starts building the front end and adds the system that puts it in place.
pub(super) fn install(app: &mut App, plugin: &FrontendPlugin) {
    let assets = crate::ui::ensure(app, &plugin.dir);
    app.world_mut().resource_mut::<UiLoading>().begin(PART);
    app.insert_resource(PendingFrontend::start(Setup::of(plugin), assets))
        .add_systems(Update, poll.in_set(FrameSet::Prepare).run_if(resource_exists::<PendingFrontend>));
}

/// Puts the front end in place once it is built. Until then Escape belongs to the front end that is coming (it does not
/// quit) and the front end decides when the scene is over, as it will once it runs.
pub(super) fn poll(
    mut commands: Commands,
    mut pending: ResMut<PendingFrontend>,
    mut loading: ResMut<UiLoading>,
    mut host: NonSendMut<Host>,
) {
    host.cancel_handled = true;
    host.flow_driven = true;
    let Some(frontend) = pending.0.take() else { return };
    commands.remove_resource::<PendingFrontend>();
    loading.end(PART);
    let Some(frontend) = frontend else {
        // The UI is off (the assets' load said why): the scene runs alone, as it did without a front end.
        host.cancel_handled = false;
        host.flow_driven = false;
        return;
    };
    commands.insert_resource(frontend);
}

#[cfg(test)]
#[path = "pending_tests.rs"]
mod tests;

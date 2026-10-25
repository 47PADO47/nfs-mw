//! What a viewer scene is, and the cameras scenes share. The window, loop and renderer belong to
//! [`crate::app`]; input arrives as actions ([`crate::input`]).

pub mod camera;
mod race_report;
mod static_hash;
pub use race_report::{
    PhotoKind, PhotoReport, PursuitReport, RaceAnswer, RaceReport, ReportLap, ReportRacer, RewardLine, RewardsReport,
    photo_picture,
};
pub use static_hash::StaticHash;

use anyhow::Result;
use blackbox_gfx::{FrameParams, Instance, RenderBackend};

use crate::hud::MapBlip;
use crate::input::ActionState;

mod fullscreen;
pub use fullscreen::{Fit, Fullscreen};

mod environment;
pub use environment::{Environment, SunMode};

/// The player's car at one moment, in render space (x forward, y left, z up; metres and seconds).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerCar {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    /// Heading, radians from +x towards +y.
    pub heading: f32,
    /// Speed along the car, m/s (negative in reverse).
    pub speed: f32,
    pub rpm: f32,
    /// -1 reverse, 0 neutral, 1.. forward gears.
    pub gear: i32,
    /// Nitrous left, 0 to 1; 0 without the part.
    pub nos: f32,
    /// Damage, 0 to 1 of the zones' levels.
    pub damage: f32,
}

/// The camera the player looks through, and the gameplay view it is in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraReadout {
    pub mode: crate::settings::CameraMode,
    pub position: [f32; 3],
    /// Unit length.
    pub forward: [f32; 3],
    /// Vertical field of view, degrees.
    pub fov: f32,
}

/// The track and the district the scene is in.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldInfo {
    pub track: String,
    /// The district at the player, `None` where the install names none.
    pub section: Option<String>,
}

pub trait Scene {
    fn extra_options(&mut self) -> Option<&mut crate::extra_options::State> {
        None
    }
    fn apply_extra_option(
        &mut self,
        _renderer: &mut dyn RenderBackend,
        _action: crate::extra_options::Action,
    ) -> Result<()> {
        anyhow::bail!("Extra options require a driving world")
    }
    fn screenshot_ui_hidden(&self) -> bool {
        false
    }

    /// Live population diagnostics for the graphics lab, where a world is present.
    fn traffic_status(&self) -> Option<String> {
        None
    }
    fn environment(&self) -> Option<Environment> {
        None
    }
    fn set_environment(&mut self, _value: Environment) {}
    /// Whether a mod has held the hour (`bb.time` freeze). Scenes without a clock report false.
    fn time_held(&self) -> bool {
        false
    }
    fn set_time_held(&mut self, _held: bool) {}
    /// The minimap blips the mods placed (`bb.map`), merged into the scene's own. Scenes without a minimap ignore them.
    fn set_mod_blips(&mut self, _blips: &[MapBlip]) {}
    /// Sets the GPS route to a point on the road (render space x, y), or clears it with `None`. Returns whether the
    /// route is set. Scenes without a road network refuse. The world map, the console and the mods come through here.
    fn set_waypoint(&mut self, _target: Option<[f32; 2]>) -> bool {
        false
    }
    /// The player's car, as the mods read it (`bb.vehicles`). `None` where no car is driven. Takes `&mut` because the
    /// car's physics is reached through the drive.
    fn player_car(&mut self) -> Option<PlayerCar> {
        None
    }
    /// The gameplay camera, as the mods read it (`bb.camera`). `None` where the scene has no camera of its own.
    fn camera_readout(&self) -> Option<CameraReadout> {
        None
    }
    /// The track and the district of the scene (`bb.world`). `None` where no world is loaded.
    fn world_info(&self) -> Option<WorldInfo> {
        None
    }
    /// Move the player's car to the road nearest `at`, facing `heading`. Returns whether there was a car to move.
    fn move_player(&mut self, _at: [f32; 2], _heading: f32) -> bool {
        false
    }
    /// Repair the player's car. Returns whether there was a car to repair.
    fn repair_player(&mut self) -> bool {
        false
    }
    /// Keep `cars` computer-driven cars on the road, `cop_share` percent of them patrol cops.
    fn set_traffic_density(&mut self, _cars: u32, _cop_share: u8) {}
    /// Put a traffic car on the road at `at` (x, y), facing `heading`. Returns whether a car was placed.
    fn spawn_traffic_at(&mut self, _at: [f32; 2], _heading: f32) -> bool {
        false
    }

    /// Installed car choices for the runtime garage; scenes without driving return None.
    fn garage(&mut self) -> Option<&mut crate::garage::Model> {
        None
    }
    fn tuning_available(&self) -> Option<bool> {
        None
    }
    fn apply_garage(&mut self, _renderer: &mut dyn RenderBackend, _selection: crate::garage::Selection) -> Result<()> {
        anyhow::bail!("this scene has no garage")
    }
    /// Start a normal garage session. Diagnostic F6 edits use `apply_garage` directly.
    fn begin_garage(&mut self, _renderer: &mut dyn RenderBackend) -> Result<()> {
        let model = self.garage().ok_or_else(|| anyhow::anyhow!("this scene has no garage"))?;
        model.reset_draft();
        Ok(())
    }
    fn begin_tuning(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.begin_garage(renderer)
    }

    /// Select the original garage platform and its authored camera shot.
    fn garage_view(
        &mut self,
        _renderer: &mut dyn RenderBackend,
        _stage: crate::garage::View,
        _camera: &'static str,
    ) -> Result<()> {
        Ok(())
    }
    /// Show an uncommitted selection. Career scenes must not debit or save here.
    fn preview_garage(&mut self, renderer: &mut dyn RenderBackend, selection: crate::garage::Selection) -> Result<()> {
        self.apply_garage(renderer, selection)
    }
    /// Validate and commit the cart after the player confirms it.
    fn confirm_garage(&mut self, renderer: &mut dyn RenderBackend, selection: crate::garage::Selection) -> Result<()> {
        self.apply_garage(renderer, selection)
    }

    /// Pause tuning saves an owned career car, but never creates a My Cars slot for a stock race car.
    fn confirm_tuning(&mut self, renderer: &mut dyn RenderBackend, selection: crate::garage::Selection) -> Result<()> {
        self.confirm_garage(renderer, selection)
    }
    /// Restore the selection at session entry.
    fn cancel_garage(&mut self, _renderer: &mut dyn RenderBackend) -> Result<()> {
        Ok(())
    }
    fn take_garage_request(&mut self) -> bool {
        self.garage().is_some_and(|model| std::mem::take(&mut model.open_requested))
    }
    fn select_garage_slot(&mut self, _renderer: &mut dyn RenderBackend, _id: &str) -> Result<()> {
        anyhow::bail!("This garage has no custom car slots")
    }
    fn add_garage_slot(&mut self, _renderer: &mut dyn RenderBackend) -> Result<()> {
        anyhow::bail!("This garage has no custom car slots")
    }
    fn delete_garage_slot(&mut self, _renderer: &mut dyn RenderBackend) -> Result<()> {
        anyhow::bail!("This garage has no custom car slots")
    }
    /// Freeze driving while the garage owns the controls, allowing streaming and placement to continue.
    fn set_garage_open(&mut self, _open: bool) {}
    fn title(&self) -> String;
    /// Upload the initial resources.
    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()>;
    /// Release scene-owned resources before replacement; free this scene's GPU resources before it is dropped. The
    /// host calls it when the scene leaves the window, unless the scene is parked for reuse. Repeated calls must be
    /// harmless.
    fn close(&mut self, _renderer: &mut dyn RenderBackend) {}
    /// The scene is put back in the window after another scene had it: re-apply the renderer state it relies on.
    fn reattached(&mut self, _renderer: &mut dyn RenderBackend) {}
    /// Advance by `dt` seconds: camera, streaming, uploads.
    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32);
    /// The frame to draw. Instances of the same mesh should be adjacent.
    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]);
    /// Refresh view-dependent offscreen captures before borrowing the main instance list.
    fn prepare_frame(&mut self, _renderer: &mut dyn RenderBackend, _aspect: f32) {}
    fn set_road_reflections(
        &mut self,
        _quality: crate::settings::RoadReflections,
        _resolution: crate::settings::RoadReflectionResolution,
    ) {
    }
    /// The rear-view mirror's frame, when the scene has a car to look back from. The mirror is drawn
    /// offscreen and shown in the HUD; the car itself is left out of it.
    /// The high-speed streaks' strength, 0 to 1 (`docs/specs/speed-effects.md`).
    fn speed_streaks(&self) -> f32 {
        0.0
    }
    fn rear_view(&mut self, _aspect: f32) -> Option<(FrameParams, &[Instance])> {
        None
    }
    /// For screenshots: whether everything the first view needs has loaded.
    fn ready(&self) -> bool {
        true
    }
    /// Whether a race is asked for or waits for its grid: the car is being put on the start and the scenery around it is
    /// streaming. The front end holds the loading screen over the world until it is over (`flow_loading.rs`).
    fn race_staging(&self) -> bool {
        false
    }
    /// Start timing once resources are ready, even while a scripted route is still moving.
    fn benchmark_ready(&self) -> bool {
        self.ready()
    }
    /// Whether the radio plays while this scene is shown without a car (the main menu's backdrop). Driving scenes
    /// leave it off: the radio plays there only while a car is driven.
    fn radio_on_menus(&self) -> bool {
        false
    }
    /// Mouse look: hide and hold the cursor so mouse motion always turns the camera.
    /// Esc releases the cursor and a click captures it again.
    fn captures_mouse(&self) -> bool {
        false
    }
    /// The scene's own egui panels, drawn every UI pass over the window (after the scene's update, before the
    /// frame is drawn). Returns the width in points of the strip on the left that they take, so the overlays
    /// can sit beside them; 0 when there is none. Scenes without panels ignore it.
    fn ui(&mut self, _ctx: &egui::Context) -> f32 {
        0.0
    }
    /// Whether [`Scene::ui`] would draw anything this frame. The UI pass is skipped on frames where no scene or
    /// developer panel is up, so a scene with conditional panels says here when none is. The default, `true`, keeps
    /// the pass running for every scene that does not say.
    fn wants_ui(&self) -> bool {
        true
    }
    /// Extra text for the window title (streaming progress, counts).
    fn status(&self) -> Option<String> {
        None
    }
    /// Lines for the scene's debug readout, drawn bottom left whatever the metrics level is: `level` says how
    /// much (the original HUD already shows speed, rpm and gear, so `Minimal` is one line that adds to it).
    /// Not called at `Off`.
    fn readout(&self, _level: crate::devtools::ShowReadout) -> Option<String> {
        None
    }
    /// Who changes gear (the transmission setting), told every frame before `update`; a scene without a car
    /// ignores it.
    fn set_transmission(&mut self, _transmission: crate::settings::Transmission) {}
    /// Which gameplay camera to use (the camera setting), told every frame; scenes without a chase camera ignore it.
    fn set_camera_mode(&mut self, _mode: crate::settings::CameraMode) {}
    /// Next camera accepted by this scene's ordinary gameplay cycle. Direct selections remain available.
    fn next_camera_mode(&self, current: crate::settings::CameraMode) -> crate::settings::CameraMode {
        current.next()
    }
    /// Whether the camera hotkey changes the gameplay camera now (only while driving).
    fn takes_camera_setting(&self) -> bool {
        false
    }
    /// A steering wheel's optional controls (the clutch pedal, the H-shifter), told every frame before `update`; a
    /// scene without a car ignores it.
    fn set_wheel_options(&mut self, _wheel: crate::settings::WheelOptions) {}
    /// How many computer-driven cars to keep around the player, what share of them are cops and whether they
    /// obey traffic lights, told every frame before `update`; a scene without roads ignores it.
    fn set_traffic(&mut self, _traffic: crate::settings::TrafficSettings) {}
    /// The second player's actions for this frame, told before `update` while a split-screen race is on.
    fn set_second_input(&mut self, _input: &ActionState) {}
    /// The device a co-op guest (player 2) drives with, told to the input layer's seating every frame; `None` outside
    /// co-op. While it is set, player 1 keeps the WASD block (or the keyboard) and the guest holds this device.
    fn coop_second_device(&self) -> Option<crate::input::CoopDevice> {
        None
    }
    /// Ends the co-op session on the next frame (the pause menu's End Co-op): player 2's car leaves the world. A scene
    /// with no co-op ignores it.
    fn end_coop(&mut self) {}
    /// Whether a co-op session may not end now: player 2 is racing or chased (`docs/specs/coop-career.md` section 3.2).
    fn coop_end_locked(&self) -> bool {
        false
    }
    /// A split-screen race is on: the picture is two views, one per player (`split_view`), each with its own HUD
    /// (`split_hud_state`), and the main `frame` draws nothing of its own.
    fn split_active(&self) -> bool {
        false
    }
    /// The frame of one player's half of the screen (0 the top, 1 the bottom), drawn at `aspect` (the half's width over
    /// its height); `None` when no split-screen race is on.
    fn split_view(&mut self, _player: usize, _aspect: f32) -> Option<(FrameParams, &[Instance])> {
        None
    }
    /// What one player's HUD shows (`split_view`'s player); `None` outside a split-screen race.
    fn split_hud_state(&self, _player: usize) -> Option<crate::hud::HudState> {
        None
    }
    /// What the in-game HUD shows this frame; `None` hides it.
    fn hud_state(&self) -> Option<crate::hud::HudState> {
        None
    }
    /// The car being driven, for the engine sound; `None` is silence.
    fn car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        None
    }
    /// The second player's car in a co-op session, for its own engine voice; `None` outside co-op.
    fn second_car_sound(&mut self) -> Option<crate::audio::CarSoundState> {
        None
    }
    /// The chase the cops run this frame, for the pursuit music and the speech; `None` without one.
    fn pursuit_feed(&self) -> Option<crate::audio::PursuitFeed> {
        None
    }
    /// What the speech machine reads of the world while no cop chases (the free-roam 911 call's inputs); `None` with a
    /// chase on. Taking it clears what the player did since the last call.
    fn roam_feed(&mut self) -> Option<crate::audio::RoamFeed> {
        None
    }
    /// The speech made the 911 call: the player's 911 timer runs `secs` seconds and the cops are let in.
    fn start_911_call(&mut self, _secs: f32) {}
    /// The speech controller's actual phase, or unavailable/muted speech.
    fn note_police_speech(&mut self, _phase: Option<blackbox_pursuit::siren::Phase>) {}
    /// How the last chase ended, while none runs: the speech says the suspect's escape once per end. `None` before any.
    fn pursuit_ended(&self) -> blackbox_pursuit::EndReason {
        blackbox_pursuit::EndReason::None
    }
    /// The bust sequence's infractions ticket, once, when the front end should show it (`(infraction, fine)` lines);
    /// `None` otherwise.
    fn take_bust_ticket(&mut self) -> Option<Vec<(blackbox_pursuit::Infraction, u32)>> {
        None
    }
    /// The finished race's results, once, when the front end should show the post-race screen; `None` otherwise.
    fn take_race_report(&mut self) -> Option<RaceReport> {
        None
    }
    /// The photo of a speed trap the car has just crossed in free roam, once, when the front end should show it.
    fn take_trap_photo(&mut self) -> Option<PhotoReport> {
        None
    }
    /// The milestones a finished chase banked, once, when the front end should show the rewards screen.
    fn take_rewards_report(&mut self) -> Option<RewardsReport> {
        None
    }
    /// Whether a race is running or staged in the world (the pause menu offers Resume Race and Restart then).
    fn race_active(&self) -> bool {
        false
    }
    /// The rap sheet's jump: the car goes to the point (render space x and y) and stands on the road nearest it. Returns
    /// whether there was a car to move.
    fn jump_player(&mut self, _at: [f32; 2]) -> bool {
        false
    }
    /// Starts the career event `event` (an event id) in the running world. Returns whether the race could start.
    fn start_career_event(&mut self, _event: &str) -> bool {
        false
    }
    /// Runs the race in progress again from its grid. Returns whether there was a race to restart.
    fn restart_race(&mut self) -> bool {
        false
    }
    /// The post-race screen's answer. `true` when a race was waiting on it.
    fn answer_race(&mut self, _answer: RaceAnswer) -> bool {
        false
    }
    /// The player pays the bust's fines on the ticket (`marker`: with a marker). `true` when the bust took the payment.
    fn pay_bust(&mut self, _marker: bool) -> bool {
        false
    }
    /// What the career asks the front end to do (open the safe house, say), once; `None` outside career mode.
    fn take_career_request(&mut self) -> Option<crate::career::CareerRequest> {
        None
    }
    /// The game is on but frozen (the pause menu is up): there is no car sound, yet the game is still being
    /// played, so the radio goes on.
    fn paused(&self) -> bool {
        false
    }
    /// Takes a multiplayer client: the world scene runs in client mode from now on and drives the session
    /// (`scenes/world/net_client/`). `false` from a scene that cannot (a menu, a viewer).
    #[cfg(feature = "multiplayer")]
    fn attach_net_client(&mut self, _client: &crate::net::ClientHandle) -> bool {
        false
    }
    /// The scene keeps running under the pause menu (a multiplayer client: the server never pauses), with neutral
    /// input.
    fn runs_while_paused(&self) -> bool {
        false
    }
    /// A menu is up over this scene (the backdrop, or the frozen world under the pause menu or the loading screen):
    /// nothing in the picture moves unless the camera or the instances do, so the game caps the frame rate there and
    /// a frame that looks like the last one is not drawn again (`app/render/static_frame.rs`). Anything that
    /// animates by itself, or changes the renderer's state while running, must answer `false`.
    fn menu_frame(&self) -> bool {
        false
    }
    /// Apply the independently layered tire-visual settings. Scenes without tires ignore them.
    fn set_tire_effects(&mut self, _smoke: bool, _skid_marks: bool) {}
    /// Apply optional collision sparks and high-speed wind trails.
    fn set_vehicle_effects(&mut self, _sparks: bool, _trails: bool) {}
    fn set_spark_style(&mut self, _style: crate::settings::SparkStyle) {}
    /// Which high-speed effect the speed trails setting shows. Scenes without a car ignore it.
    fn set_speed_trail_style(&mut self, _style: crate::settings::SpeedTrailStyle) {}
    /// Show the flames at the car's tail pipes (the `exhaust_flames` setting). Scenes without a car ignore it.
    fn set_exhaust_flames(&mut self, _on: bool) {}
    /// The sound's sputter module started `pops` pops since the last call (the lift-off backfire follows them).
    fn note_sputters(&mut self, _pops: u32) {}
    /// Refresh changed visual-effect buffers without advancing a paused scene.
    fn refresh_effects(&mut self, _renderer: &mut dyn RenderBackend) {}
    /// Apply the optional smoke presentation quality. Scenes without tires ignore it.
    fn set_smoke_quality(&mut self, _quality: crate::settings::SmokeQuality) {}
    /// Apply the car shading setting: the scene re-uploads its cars when it changes. Scenes without cars ignore it.
    fn set_detail(&mut self, _detail: crate::settings::DetailSettings) {}
    fn set_car_shading(&mut self, _shading: crate::settings::CarShading) {}
    /// Rain on every track, and its amount (0 to 1). Scenes without a road ignore it.
    fn set_rain(&mut self, _always: bool, _amount: f32) {}
    /// The world's speeds: the physics clock and the animation clock run at these multiples of the original rates. Scenes without a world ignore it.
    fn set_world_tuning(&mut self, _tuning: crate::settings::WorldTuning) {}
    /// A picture to show over the window this frame, if any.
    fn fullscreen(&mut self) -> Option<Fullscreen> {
        None
    }
    /// Sound the scene wants played once, handed over when asked (the audio of a movie).
    fn take_clip(&mut self) -> Option<ea_audio::Pcm> {
        None
    }
    /// The one-shot sounds the traffic cars asked for since the last call (horns, near-miss whooshes).
    fn take_traffic_sounds(&mut self) -> Vec<crate::audio::traffic::TrafficSound> {
        Vec::new()
    }
    /// The police helicopter's rotor for this frame (`dt` seconds since the last one), or `None` without one.
    fn rotor_frame(&mut self, _dt: f32) -> Option<crate::audio::HeliFrame> {
        None
    }
    /// The police cars' sirens for this frame (`dt` seconds since the last one), or `None` without any police.
    fn siren_frame(&mut self, _dt: f32) -> Option<crate::audio::SirenFrame> {
        None
    }
    /// The NIS player's audio request: the scene whose stream plays now, if any (see `audio::nis`). Scenes without
    /// NIS ignore it.
    fn nis_audio(&self) -> Option<&crate::audio::nis::NisAudioRequest> {
        None
    }
    /// Whether the NIS audio still plays after this frame's update. Called every frame. Scenes without NIS ignore it.
    fn note_nis_audio(&mut self, _playing: bool) {}
    /// Positions of petrol-station blasts since the last call, in render space.
    fn take_explosions(&mut self) -> Vec<glam::Vec3> {
        Vec::new()
    }
    fn take_world_effect_audio(&mut self) -> Vec<crate::audio::WorldEffectAudio> {
        Vec::new()
    }
    /// The sound viewer's transport, when the scene has one: the audio plugin plays what it asks for.
    fn sound_preview(&self) -> Option<crate::audio::preview::Preview> {
        None
    }
    /// Ordered global sound events since the last update.
    fn take_common_sounds(&mut self) -> Vec<crate::audio::CommonEvent> {
        Vec::new()
    }
    /// The scene is over: the app quits.
    fn finished(&self) -> bool {
        false
    }
    /// The scene's own console commands as `(name, usage)`, listed by `help`.
    fn commands(&self) -> &'static [(&'static str, &'static str)] {
        &[]
    }
    /// Run a console command. `None` means the scene has no such command.
    fn command(
        &mut self,
        _renderer: &mut dyn RenderBackend,
        _name: &str,
        _args: &[&str],
    ) -> Option<Result<String, String>> {
        None
    }
}

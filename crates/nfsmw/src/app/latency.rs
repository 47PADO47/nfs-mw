//! Sleep before time/input sampling; frame IDs and presentation markers belong to the renderer.
use super::Host;
use crate::settings::Settings;
use bevy_ecs::prelude::*;
use bevy_window::{PrimaryWindow, Window};

pub(super) fn begin(mut host: NonSendMut<Host>, settings: Res<Settings>, window: Single<&Window, With<PrimaryWindow>>) {
    let _trace = blackbox_trace::scope("Wait: Reflex / driver pacing");
    let host = &mut *host;
    let gameplay = host.screenshot.is_none()
        && host.scene.takes_camera_setting()
        && !host.scene.paused()
        && host.scene.benchmark_ready()
        && window.focused;
    let started = std::time::Instant::now();
    host.latency_pacing = host.renderer.as_mut().is_some_and(|renderer| {
        renderer.begin_latency_frame(blackbox_gfx::LatencySettings {
            reflex: settings.reflex,
            generated_frames: settings.frame_generation.0,
            frame_limit_us: match settings.vsync {
                true => 0,
                false => settings.max_fps.microseconds(),
            },
            gameplay,
        })
    });
    host.reflex_sleep_ms = started.elapsed().as_secs_f32() * 1000.0;
    host.cpu_work_started = std::time::Instant::now();
}

pub(super) fn input(mut host: NonSendMut<Host>, mouse: Res<bevy_input::ButtonInput<bevy_input::mouse::MouseButton>>) {
    if mouse.just_pressed(bevy_input::mouse::MouseButton::Left)
        && let Some(renderer) = host.renderer.as_mut()
    {
        renderer.latency_input();
    }
}

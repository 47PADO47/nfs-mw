//! Raw observations, independent of overlay smoothing and classifications.
use crate::{app::Host, settings::Settings};
use blackbox_gfx::RenderBackend;
use std::time::Instant;

#[derive(Default, Debug, Clone)]
pub(crate) struct Samples {
    session: u64,
    settings: Option<Settings>,
    gpu_sample: u64,
    activity_sample: u64,
    observation: u64,
    last_observation: Option<Instant>,
}

pub(crate) fn observe(
    state: &mut Samples,
    renderer: &dyn RenderBackend,
    host: &Host,
    settings: &Settings,
    focused: bool,
) {
    let trace = blackbox_trace::global();
    let session = trace.session();
    if session == 0 {
        return;
    }
    if state.session != session {
        *state = Samples { session, ..Default::default() };
        trace.note("Backend", &format!("{:?}; debug_assertions={}", renderer.info(), cfg!(debug_assertions)));
    }
    if state.settings.as_ref() != Some(settings) {
        state.settings = Some(*settings);
        let text: Vec<char> = format!("{settings:#?}").chars().collect();
        for (index, part) in text.chunks(3500).enumerate() {
            trace.note("Settings", &format!("Part {index}\n{}", part.iter().collect::<String>()));
        }
        trace.note("Effective graphics", &format!("{:?}", renderer.graphics()));
    }
    let performance = renderer.frame_performance();
    // Backends without FramePerformance support return defaults; don't graph those zeros as measurements.
    let render_timing = |value: f32| match performance.rendered_frames > 0 {
        true => f64::from(value),
        false => f64::NAN,
    };
    let latency = renderer.latency_status();
    let [ow, oh] = renderer.surface_size();
    let [rw, rh] = renderer.render_size();
    let now = Instant::now();
    let elapsed = state.last_observation.replace(now).map(|last| now.duration_since(last).as_secs_f64() * 1000.0);
    state.observation += 1;
    trace.counter(
        "Frame observations",
        &[
            ("observation_id", state.observation as f64),
            ("frame_interval_ms", elapsed.unwrap_or(f64::NAN)),
            (
                "renderer_base_frame_id",
                match renderer.info().renderer {
                    "blackbox" => performance.rendered_frames as f64,
                    _ => f64::NAN,
                },
            ),
            ("cpu_update_elapsed_ms", f64::from(host.cpu_update_ms)),
            ("cpu_render_elapsed_ms", render_timing(performance.cpu_render_ms)),
            ("acquire_wait_ms", render_timing(performance.acquire_wait_ms)),
            ("present_wait_ms", render_timing(performance.present_wait_ms)),
            ("driver_pacing_ms", f64::from(host.reflex_sleep_ms)),
            ("focused", u8::from(focused).into()),
            ("output_width", ow.into()),
            ("output_height", oh.into()),
            ("render_width", rw.into()),
            ("render_height", rh.into()),
        ],
    );
    trace.counter(
        "Presentation observations",
        &[
            ("sdk_presented_total", latency.presented_total.map_or(f64::NAN, |v| v as f64)),
            ("fg_active", u8::from(latency.generation_active).into()),
            ("driver_pacing", u8::from(latency.driver_pacing).into()),
        ],
    );
    if performance.gpu_sample != state.gpu_sample {
        state.gpu_sample = performance.gpu_sample;
        trace.counter(
            "GPU base render (delayed readback)",
            &[
                ("sample_id", performance.gpu_sample as f64),
                ("duration_ms", performance.gpu_render_ms.map_or(f64::NAN, f64::from)),
            ],
        );
    }
    let activity = performance.activity;
    if activity.sample == state.activity_sample {
        return;
    }
    state.activity_sample = activity.sample;
    trace.note("GPU activity engine", activity.gpu_engine);
    trace.counter(
        "OS activity (sampled interval)",
        &[
            ("sample_id", activity.sample as f64),
            ("age_ms", activity.age_ms.into()),
            ("main_thread_active_percent", activity.main_thread_busy_percent.map_or(f64::NAN, f64::from)),
            ("busiest_process_gpu_engine_percent", activity.gpu_busy_percent.map_or(f64::NAN, f64::from)),
        ],
    );
}

//! Capture lifecycle and hotkeys. Rendering never writes files or waits for the export worker.
mod health;
pub(crate) use health::process_counters;
mod identity;
mod phases;
mod samples;
#[cfg(test)]
mod tests;

use super::{GraphicsLab, logbuf};
use bevy_app::{App, First, Last, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_input::{ButtonInput, InputSystems, keyboard::KeyCode};
use bevy_window::{PrimaryWindow, Window};
use std::path::PathBuf;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(super) struct Recorder {
    pub seconds: u32,
    started: Option<Instant>,
    writer: Option<JoinHandle<Result<PathBuf, String>>>,
    pub message: String,
    pub last_path: Option<PathBuf>,
    bookmarks: u32,
    last_counts: (usize, u64),
    completed: Option<Instant>,
}

impl Default for Recorder {
    fn default() -> Self {
        Self {
            seconds: 15,
            started: None,
            writer: None,
            message: "Ready — F9 record / F10 bookmark".into(),
            last_path: None,
            bookmarks: 0,
            last_counts: (0, 0),
            completed: None,
        }
    }
}

impl Recorder {
    pub(super) fn command(&mut self, args: &[String]) -> Result<String, String> {
        match args.first().map(String::as_str) {
            Some("start") if args.len() <= 2 => {
                if self.recording() || self.writer.is_some() {
                    return Err("A trace is recording or saving".into());
                }
                if let Some(value) = args.get(1) {
                    self.seconds = value
                        .parse::<u32>()
                        .ok()
                        .filter(|s| (1..=60).contains(s))
                        .ok_or("Trace duration must be 1–60 seconds")?;
                }
                self.toggle();
            }
            Some("stop") if args.len() == 1 => self.stop(),
            Some("mark") => {
                if !self.recording() {
                    return Err("Start a trace before adding a bookmark".into());
                }
                self.bookmark();
                if args.len() > 1 {
                    blackbox_trace::global().note("Bookmark label", &args[1..].join(" "));
                }
            }
            None | Some("status") if args.len() <= 1 => {}
            _ => return Err("trace start [1–60 seconds] | stop | mark [label] | status".into()),
        }
        Ok(format!("Trace: {}", self.message))
    }

    pub fn recording(&self) -> bool {
        self.started.is_some()
    }

    pub fn toggle(&mut self) {
        if self.recording() {
            self.stop();
            return;
        }
        if self.writer.is_some() {
            return;
        }
        if !blackbox_trace::global().start(blackbox_trace::MAX_EVENTS) {
            self.message = "Another capture is already active".into();
            return;
        }
        self.started = Some(Instant::now());
        self.bookmarks = 0;
        self.completed = None;
        self.message = format!("Recording {} seconds — F9 stop / F10 bookmark", self.seconds);
        blackbox_trace::global().note("Capture", concat!("nfsmw ", env!("CARGO_PKG_VERSION")));
        blackbox_trace::global().note("Timing scope", "CPU scopes measure elapsed wall time including waits. GPU counters are delayed readbacks at observation time, not a calibrated GPU timeline. FG internals are not instrumented.");
        logbuf::output(&self.message);
    }

    pub fn bookmark(&mut self) {
        if !self.recording() {
            return;
        }
        self.bookmarks += 1;
        blackbox_trace::global().note("Bookmark", &format!("User bookmark {}", self.bookmarks));
        self.message = format!("Recording — bookmark {} added", self.bookmarks);
    }

    fn stop(&mut self) {
        let Some(started) = self.started.take() else { return };
        let ended = Instant::now();
        let ended_unix_ms = health::unix_ms();
        let Some(capture) = blackbox_trace::global().stop() else { return };
        self.last_counts = (capture.events(), capture.dropped());
        self.message = format!("Saving {} events ({} dropped)…", capture.events(), capture.dropped());
        let path = capture_path();
        match std::thread::Builder::new().name("performance-trace-writer".into()).spawn(move || {
            let path = path.ok_or("No per-user log directory is available")?;
            let saved = save(capture, path)?;
            identity::save_context(&saved, started, ended, ended_unix_ms)?;
            Ok(saved)
        }) {
            Ok(writer) => self.writer = Some(writer),
            Err(error) => self.message = format!("Could not start trace writer: {error}"),
        }
    }

    fn poll(&mut self) {
        if !self.writer.as_ref().is_some_and(|writer| writer.is_finished()) {
            return;
        }
        let Some(writer) = self.writer.take() else { return };
        match writer.join().unwrap_or_else(|_| Err("Trace writer panicked".into())) {
            Ok(path) => {
                self.message = format!(
                    "Saved trace: {} ({} events, {} dropped)",
                    path.display(),
                    self.last_counts.0,
                    self.last_counts.1
                );
                self.last_path = Some(path);
                self.completed = Some(Instant::now());
                logbuf::output(&self.message);
            }
            Err(error) => {
                self.message = format!("Trace export failed: {error}");
                self.completed = Some(Instant::now());
                logbuf::error(&self.message);
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Performance trace");
        ui.horizontal_wrapped(|ui| {
            let label = match self.recording() {
                true => "Stop & save (F9)",
                false => "Record (F9)",
            };
            if ui.add_enabled(self.writer.is_none(), egui::Button::new(label)).clicked() {
                self.toggle();
            }
            if ui.add_enabled(self.recording(), egui::Button::new("Bookmark (F10)")).clicked() {
                self.bookmark();
            }
            ui.add_enabled_ui(!self.recording(), |ui| {
                egui::ComboBox::from_id_salt("trace_seconds")
                    .selected_text(format!("{} seconds", self.seconds))
                    .show_ui(ui, |ui| {
                        for seconds in [5, 15, 30, 60] {
                            ui.selectable_value(&mut self.seconds, seconds, format!("{seconds} seconds"));
                        }
                    });
            });
        });
        ui.small("Frame/wait timings, hitch markers, GPU/activity, resources and memory. A companion context JSON includes build identity and up to ten minutes of memory history. Export runs in the background.");
        ui.label(&self.message);
        if let Some(path) = &self.last_path {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Copy trace path").clicked() {
                    ui.ctx().copy_text(path.display().to_string());
                }
                ui.hyperlink_to("Open Perfetto viewer", "https://ui.perfetto.dev");
            });
        }
        ui.small("Open the saved JSON file in Perfetto. No trace is uploaded automatically. Hotkeys work while the game window is focused, including during gameplay.");
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop();
        // Only shutdown waits; never join the writer during a live frame.
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
    }
}

fn capture_path() -> Option<PathBuf> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_nanos();
    Some(
        game_install::log_dir(&nfsmw_data::game::SPEC)?
            .join("traces")
            .join(format!("trace-{stamp}-{}.json", std::process::id())),
    )
}

fn save(capture: blackbox_trace::Capture, path: PathBuf) -> Result<PathBuf, String> {
    let result = (|| -> std::io::Result<()> {
        let parent = path.parent().ok_or_else(|| std::io::Error::other("Missing trace directory"))?;
        std::fs::create_dir_all(parent)?;
        let temporary = path.with_extension("json.partial");
        let file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
        capture.write_json(std::io::BufWriter::new(file))?;
        std::fs::rename(temporary, &path)
    })();
    result.map(|()| path).map_err(|error| error.to_string())
}

#[derive(Default)]
pub(crate) struct FrameScope(Option<blackbox_trace::Scope<'static>>);

pub(super) fn build(app: &mut App) {
    phases::build(app);
    app.insert_non_send(FrameScope::default())
        .add_systems(First, begin.before(bevy_time::TimeSystems))
        .add_systems(PreUpdate, hotkeys.after(InputSystems))
        .add_systems(Last, end.after(crate::app::pacing::end_of_frame));
}

pub(crate) fn begin(mut frame: NonSendMut<FrameScope>) {
    frame.0 = Some(blackbox_trace::scope("Frame"));
}

pub(super) fn indicator(ctx: &egui::Context, recorder: &Recorder) {
    if !recorder.recording()
        && recorder.writer.is_none()
        && !recorder.completed.is_some_and(|at| at.elapsed() < Duration::from_secs(8))
    {
        return;
    }
    egui::Area::new(egui::Id::new("trace_status"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::window(&ctx.global_style()).show(ui, |ui| {
                if let Some(start) = recorder.started {
                    ui.colored_label(
                        egui::Color32::LIGHT_RED,
                        format!(
                            "● Trace {:.1}/{} s · F9 stop · F10 bookmark",
                            start.elapsed().as_secs_f32(),
                            recorder.seconds
                        ),
                    );
                    return;
                }
                if recorder.writer.is_some() {
                    ui.label("Saving performance trace…");
                    return;
                }
                ui.label("Trace finished — F8 Diagnostics for path/status");
            });
        });
}

pub(super) use samples::Samples;
pub(super) use samples::observe;

fn end(mut frame: NonSendMut<FrameScope>, mut lab: ResMut<GraphicsLab>) {
    frame.0.take();
    let recorder = &mut lab.recorder;
    if recorder.started.is_some_and(|start| start.elapsed() >= Duration::from_secs(u64::from(recorder.seconds))) {
        recorder.stop();
    }
    recorder.poll();
}

fn hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut lab: ResMut<GraphicsLab>,
) {
    if !window.focused {
        return;
    }
    if keys.just_pressed(KeyCode::F9) {
        lab.recorder.toggle();
    }
    if keys.just_pressed(KeyCode::F10) {
        lab.recorder.bookmark();
    }
}

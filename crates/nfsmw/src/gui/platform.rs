//! Consume egui clipboard/browser commands on a bounded worker, with visible completion/error feedback.
use super::UiOutput;
use bevy_ecs::prelude::*;
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::time::{Duration, Instant};

struct Worker {
    send: SyncSender<egui::OutputCommand>,
    results: Receiver<Result<&'static str, String>>,
}

#[derive(Default)]
pub(crate) struct Platform {
    worker: Option<Worker>,
    message: Option<(String, bool, Instant)>,
}

impl Platform {
    fn request(&mut self, command: egui::OutputCommand) {
        if self.worker.is_none() {
            match worker() {
                Ok(worker) => self.worker = Some(worker),
                Err(error) => {
                    self.feedback(Err(error));
                    return;
                }
            }
        }
        let Some(worker) = &self.worker else { return };
        if let Err(error) = worker.send.try_send(command) {
            self.feedback(Err(format!("UI platform request could not be queued: {error}")));
        }
    }

    fn feedback(&mut self, result: Result<&'static str, String>) {
        let error = result.is_err();
        let message = match result {
            Ok(message) => message.to_owned(),
            Err(error) => error,
        };
        if error {
            log::warn!("{message}");
        }
        self.message = Some((message, error, Instant::now()));
    }

    pub fn show(&self, ctx: &egui::Context) {
        let Some((message, error, at)) = &self.message else { return };
        if at.elapsed() > Duration::from_secs(8) {
            return;
        }
        egui::Area::new(egui::Id::new("platform_feedback"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::window(&ctx.global_style()).show(ui, |ui| {
                    let color = match error {
                        true => egui::Color32::LIGHT_RED,
                        false => egui::Color32::LIGHT_GREEN,
                    };
                    ui.colored_label(color, message);
                });
            });
    }
}

pub(super) fn apply(mut platform: NonSendMut<Platform>, mut out: ResMut<UiOutput>) {
    let _trace = blackbox_trace::scope("UI: dispatch platform commands");
    for command in out.platform_commands.drain(..) {
        platform.request(command);
    }
    loop {
        let result = platform.worker.as_ref().and_then(|worker| worker.results.try_recv().ok());
        let Some(result) = result else { break };
        platform.feedback(result);
    }
}

fn worker() -> Result<Worker, String> {
    let (send, commands) = sync_channel(8);
    let (results, receive) = sync_channel(8);
    std::thread::Builder::new()
        .name("ui-platform-output".into())
        .spawn(move || {
            // Retain clipboard ownership for X11 desktops; create it only on the first copy request.
            let mut clipboard = None;
            while let Ok(command) = commands.recv() {
                let _trace = blackbox_trace::scope("UI platform worker");
                let result = execute(command, &mut clipboard);
                if results.send(result).is_err() {
                    break;
                }
            }
        })
        .map_err(|error| error.to_string())?;
    Ok(Worker { send, results: receive })
}

fn execute(command: egui::OutputCommand, clipboard: &mut Option<arboard::Clipboard>) -> Result<&'static str, String> {
    match command {
        egui::OutputCommand::CopyText(text) => {
            if clipboard.is_none() {
                *clipboard =
                    Some(arboard::Clipboard::new().map_err(|error| format!("Clipboard unavailable: {error}"))?);
            }
            clipboard
                .as_mut()
                .ok_or("Clipboard unavailable")?
                .set_text(text)
                .map_err(|error| format!("Could not copy text: {error}"))?;
            Ok("Copied text to clipboard")
        }
        egui::OutputCommand::OpenUrl(url) => {
            // UI-generated web links only. Do not dispatch arbitrary executable/custom URL handlers.
            if !url.url.starts_with("https://") && !url.url.starts_with("http://") {
                return Err("Only HTTP/HTTPS browser links are supported".into());
            }
            open::that(&url.url).map_err(|error| format!("Could not open browser: {error}"))?;
            Ok("Opened link in your default browser")
        }
        _ => Err("This UI platform command is not supported".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_handlers_are_rejected_without_launching_them() {
        let command = egui::OutputCommand::OpenUrl(egui::OpenUrl::new_tab("file:///example.exe"));
        assert!(execute(command, &mut None).is_err());
    }

    #[test]
    fn egui_output_keeps_clipboard_and_browser_requests_for_the_host() {
        let gui = super::super::Gui::default();
        let mut out = UiOutput::default();
        gui.frame(egui::RawInput::default(), 1.0, &mut out, |ctx| {
            ctx.copy_text("trace path".into());
            ctx.open_url(egui::OpenUrl::new_tab("https://ui.perfetto.dev"));
        });
        assert!(
            out.platform_commands
                .iter()
                .any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text == "trace path"))
        );
        assert!(out.platform_commands.iter().any(
            |command| matches!(command, egui::OutputCommand::OpenUrl(url) if url.url == "https://ui.perfetto.dev")
        ));
    }
}

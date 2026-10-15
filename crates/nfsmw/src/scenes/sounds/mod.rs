//! The sound viewer (`sound-viewer`): every sound bank sound in the install in a searchable list on the left, and a
//! mini player at the bottom centre. Clicking a sound plays it. The scene only asks: the audio plugin plays the
//! request and reports the playhead back, through a [`Preview`] (`audio::preview`).

mod entries;
mod transport;

use anyhow::{Result, anyhow, bail};
use blackbox_gfx::{FrameParams, Instance, RenderBackend};
use game_install::GameDir;

use crate::audio::preview::{Preview, Request, Status};
use crate::frontend::MenuScene;
use crate::input::ActionState;
use crate::scenes::sidebar;
use crate::viewer::Scene;
use entries::{Entry, Listing};

pub struct SoundsScene {
    entries: Vec<Entry>,
    names: Vec<String>,
    /// Index into `entries` of the sound on the mini player, if one was picked.
    selected: Option<usize>,
    panel: sidebar::Panel,
    /// Whether Ctrl+B has hidden the list.
    collapsed: bool,
    /// The transport: asks go out, and the playhead comes back.
    preview: Preview,
    /// The last status the audio plugin reported, read once per pass.
    status: Status,
    /// The slider's value while the user drags it; the playhead otherwise.
    seek: f64,
    seeking: bool,
    /// Banks that could not be read, shown in the list header.
    unreadable: Vec<String>,
    /// The menus' flat backdrop: nothing is turned or followed.
    backdrop: MenuScene,
}

impl SoundsScene {
    /// Lists every sound of the install and starts on the bank `start` names (in any case, with or without its
    /// folder), or the first sound.
    pub fn open(dir: &GameDir, start: Option<&str>) -> Result<Self> {
        let Listing { entries, unreadable } = entries::list(dir);
        if entries.is_empty() {
            bail!("the install has no sound banks under SOUND/");
        }
        let selected = match start {
            Some(wanted) => Some(
                entries::start(&entries, Some(wanted))
                    .ok_or_else(|| anyhow!("no sound bank {wanted:?} in the install"))?,
            ),
            None => None,
        };
        let names = entries::names(&entries);
        Ok(Self {
            entries,
            names,
            selected,
            panel: sidebar::Panel::default(),
            collapsed: false,
            preview: Preview::default(),
            status: Status::default(),
            seek: 0.0,
            seeking: false,
            unreadable,
            backdrop: MenuScene::new(None),
        })
    }

    /// The sound on the mini player, when one was picked.
    pub fn selected_entry(&self) -> Option<&Entry> {
        self.selected.map(|index| &self.entries[index])
    }

    /// Asks the audio plugin to play `index` and remembers it as the selection.
    fn play(&mut self, index: usize) {
        let entry = &self.entries[index];
        self.selected = Some(index);
        self.preview.request(Request::Play { bank: entry.bank.clone(), index: entry.index });
    }

    /// The position in the list the search shows of the selected sound, if it is shown.
    fn shown(&self) -> Vec<usize> {
        sidebar::filter(&self.names, &self.panel.query)
    }

    /// Plays the sound before or after the selected one in the shown list. With nothing selected, the first one.
    fn step(&mut self, forward: bool) {
        let shown = self.shown();
        if shown.is_empty() {
            return;
        }
        let at = self.selected.and_then(|sel| shown.iter().position(|&i| i == sel));
        let next = match (at, forward) {
            (Some(at), true) => (at + 1).min(shown.len() - 1),
            (Some(at), false) => at.saturating_sub(1),
            (None, true) => 0,
            (None, false) => shown.len() - 1,
        };
        self.play(shown[next]);
    }

    /// Reads the plugin's report and keeps the slider in step with the playhead, unless the user drags it.
    fn read_status(&mut self) {
        self.status = self.preview.status();
        if !self.seeking {
            self.seek = self.status.position;
        }
    }
}

impl Scene for SoundsScene {
    fn title(&self) -> String {
        "Sound viewer".to_owned()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.backdrop.init(renderer)
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        self.backdrop.update(renderer, input, dt);
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        self.backdrop.frame(aspect)
    }

    fn sound_preview(&self) -> Option<Preview> {
        Some(self.preview.clone())
    }

    fn ui(&mut self, ctx: &egui::Context) -> f32 {
        if sidebar::toggle_pressed(ctx) {
            self.collapsed = !self.collapsed;
        }
        self.read_status();
        if self.collapsed {
            return 0.0;
        }
        let error = (!self.unreadable.is_empty()).then(|| format!("{} banks could not be read", self.unreadable.len()));
        let clicked = sidebar::show(
            ctx,
            &mut self.panel,
            &self.names,
            self.selected.unwrap_or(usize::MAX),
            error.as_deref(),
            "sounds",
        );
        if let Some(index) = clicked {
            self.play(index);
        }
        if !ctx.egui_wants_keyboard_input() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space)) {
                self.preview.request(Request::TogglePause);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)) {
                self.step(true);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)) {
                self.step(false);
            }
        }
        let name = self.selected_entry().map(Entry::name);
        let action = transport::show(
            ctx,
            name.as_deref(),
            &self.status,
            &mut self.seek,
            &mut self.seeking,
            f64::from(sidebar::width(ctx)),
        );
        match action {
            transport::Action::None => {}
            transport::Action::Previous => self.step(false),
            transport::Action::Next => self.step(true),
            transport::Action::TogglePause => self.preview.request(Request::TogglePause),
            transport::Action::Seek(at) => self.preview.request(Request::Seek(at)),
        }
        sidebar::width(ctx)
    }
}

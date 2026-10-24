//! The menu follows the session once a frame: the events go to the feed and the results, and the page follows the
//! status (seated: the lobby; over: back to the browser with the reason, or the password prompt when one is needed).

use nfsmw_net::client_session::{EndReason, SessionEvent};

use super::cx::Cx;
use super::{LanMenu, Page};
use crate::frontend::lan::text::disconnect_text;
use crate::frontend::lan::{Disconnect, NetStatus};
use crate::i18n::t;

impl LanMenu {
    /// One frame: `dt` seconds passed. The feed takes the events, a result record opens the results page.
    pub fn frame(&mut self, cx: &mut Cx, dt: f32) {
        self.feed.tick(dt);
        let events = cx.net.take_events();
        for event in &events {
            self.feed.take(event, cx.net.lobby(), &self.mutes);
            if let SessionEvent::Results(results) = event {
                self.results = Some(results.clone());
                self.results_cursor = Default::default();
                // The results show over the world, as the original's post-race page does.
                self.open = true;
                self.page = Page::Results;
            }
        }
        let status = cx.net.status();
        if status != self.last_status {
            self.follow(&status);
            self.last_status = status;
        }
    }

    fn follow(&mut self, status: &NetStatus) {
        let in_session = matches!(self.page, Page::Lobby | Page::Settings | Page::Player);
        match status {
            // Seated: the lobby shows, also for a session started from the console or the command line.
            NetStatus::InLobby if matches!(self.page, Page::Home | Page::Host | Page::Browser) => {
                self.error = None;
                self.open = true;
                self.page = Page::Lobby;
            }
            NetStatus::Disconnected(Disconnect::Session(EndReason::PasswordRequired)) => {
                self.page = Page::Browser;
                if let Some((addr, name)) = self.browser.last.clone() {
                    self.browser.ask_password(addr, name);
                }
                self.error = Some(t!("lan-browser-needs-password"));
            }
            NetStatus::Disconnected(why) if self.open => {
                self.error = Some(disconnect_text(why));
                self.page = match self.page {
                    Page::Host => Page::Host,
                    _ => Page::Browser,
                };
            }
            NetStatus::Idle if in_session => self.page = Page::Home,
            _ => {}
        }
    }
}

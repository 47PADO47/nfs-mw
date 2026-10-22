//! The Join page (MP-20): the LAN games a discovery scan found, Refresh, direct connect by address, and the password
//! prompt of a locked room (also opened when a server answers that it needs one).

use std::net::SocketAddr;

use super::cx::Cx;
use super::field::{Field, Typed};
use super::rows::{Cursor, Row, RowKind};
use crate::frontend::lan::address::socket_of;
use crate::frontend::lan::text::server_line;

/// What Accept or Back on the page did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserGo {
    Stay,
    Back,
    /// Something was refused: the page shows why.
    Error(String),
}

/// The password prompt of one server.
#[derive(Clone, Debug, PartialEq)]
pub struct Prompt {
    pub addr: SocketAddr,
    pub name: String,
    pub password: Field,
    pub cursor: Cursor,
}

/// The page's state.
#[derive(Clone, Debug, PartialEq)]
pub struct Browser {
    pub cursor: Cursor,
    pub address: Field,
    pub prompt: Option<Prompt>,
    /// The server connected to last, for a password asked after the fact.
    pub last: Option<(SocketAddr, String)>,
}

impl Default for Browser {
    fn default() -> Self {
        Self { cursor: Cursor::default(), address: Field::new("", 64), prompt: None, last: None }
    }
}

const PROMPT_ROWS: usize = 3;

impl Browser {
    /// Whether a text row has the keys.
    pub fn typing(&self) -> bool {
        self.address.editing || self.prompt.as_ref().is_some_and(|prompt| prompt.password.editing)
    }

    /// Asks for the password of `addr` (named `name`).
    pub fn ask_password(&mut self, addr: SocketAddr, name: String) {
        let mut password = Field::secret(24);
        password.editing = true;
        self.prompt = Some(Prompt { addr, name, password, cursor: Cursor::default() });
    }

    fn len(&self, cx: &Cx) -> usize {
        match &self.prompt {
            Some(_) => PROMPT_ROWS,
            None => cx.net.servers().len() + 3,
        }
    }

    pub fn rows(&self, cx: &Cx) -> Vec<Row> {
        if let Some(prompt) = &self.prompt {
            return vec![
                Row::text("Password", prompt.password.shown(), prompt.password.editing),
                Row::action("Join", true),
                Row::action("Cancel", true),
            ];
        }
        let mut rows: Vec<Row> = cx
            .net
            .servers()
            .iter()
            .map(|entry| {
                let (name, value) = server_line(entry);
                Row { label: name, value, enabled: entry.compatible, kind: RowKind::Info }
            })
            .collect();
        rows.push(Row::action("Refresh", true));
        let shown = match (self.address.editing, self.address.text.is_empty()) {
            (false, true) => "type an address".to_owned(),
            _ => self.address.shown(),
        };
        rows.push(Row::text("Direct connect", shown, self.address.editing));
        rows.push(Row::action("Back", true));
        rows
    }

    /// The selection within the rows shown now.
    pub fn selected(&self, cx: &Cx) -> usize {
        match &self.prompt {
            Some(prompt) => prompt.cursor.clamped(PROMPT_ROWS),
            None => self.cursor.clamped(self.len(cx)),
        }
    }

    pub fn up(&mut self, cx: &Cx) {
        let len = self.len(cx);
        match &mut self.prompt {
            Some(prompt) => prompt.cursor.up(len),
            None => self.cursor.up(len),
        }
    }

    pub fn down(&mut self, cx: &Cx) {
        let len = self.len(cx);
        match &mut self.prompt {
            Some(prompt) => prompt.cursor.down(len),
            None => self.cursor.down(len),
        }
    }

    pub fn take(&mut self, key: Typed) {
        self.address.take(key);
        if let Some(prompt) = &mut self.prompt {
            prompt.password.take(key);
        }
    }

    /// Back: ends typing, closes the prompt, or leaves the page.
    pub fn back(&mut self) -> BrowserGo {
        if self.address.editing {
            self.address.editing = false;
            return BrowserGo::Stay;
        }
        if self.prompt.take().is_some() {
            return BrowserGo::Stay;
        }
        BrowserGo::Back
    }

    pub fn accept(&mut self, cx: &mut Cx) -> BrowserGo {
        if self.prompt.is_some() {
            return self.accept_prompt(cx);
        }
        let servers = cx.net.servers().len();
        let at = self.cursor.clamped(self.len(cx));
        if at < servers {
            let entry = cx.net.servers()[at].clone();
            if !entry.compatible {
                return BrowserGo::Error(format!("{} runs another version of the game", entry.name));
            }
            if entry.password {
                self.ask_password(entry.addr, entry.name);
                return BrowserGo::Stay;
            }
            return self.connect(cx, entry.addr, entry.name, None);
        }
        match at - servers {
            0 => {
                cx.net.scan();
                BrowserGo::Stay
            }
            1 => self.direct(cx),
            _ => BrowserGo::Back,
        }
    }

    /// The Direct connect row: starts typing, or connects to what was typed.
    fn direct(&mut self, cx: &mut Cx) -> BrowserGo {
        if !self.address.editing {
            self.address.editing = true;
            return BrowserGo::Stay;
        }
        self.address.editing = false;
        if self.address.value().is_empty() {
            return BrowserGo::Stay;
        }
        let addr = match socket_of(self.address.value(), cx.settings.net.port) {
            Ok(addr) => addr,
            Err(e) => return BrowserGo::Error(e),
        };
        self.connect(cx, addr, addr.to_string(), None)
    }

    fn accept_prompt(&mut self, cx: &mut Cx) -> BrowserGo {
        let Some(prompt) = self.prompt.as_mut() else { return BrowserGo::Stay };
        match prompt.cursor.clamped(PROMPT_ROWS) {
            0 if !prompt.password.editing => {
                prompt.password.editing = true;
                BrowserGo::Stay
            }
            // Accept while typing the password joins at once.
            0 | 1 => {
                let Some(prompt) = self.prompt.take() else { return BrowserGo::Stay };
                let password = prompt.password.value().to_owned();
                self.connect(cx, prompt.addr, prompt.name, Some(&password))
            }
            _ => {
                self.prompt = None;
                BrowserGo::Stay
            }
        }
    }

    fn connect(&mut self, cx: &mut Cx, addr: SocketAddr, name: String, password: Option<&str>) -> BrowserGo {
        self.last = Some((addr, name));
        match cx.net.connect(addr, password) {
            Ok(()) => BrowserGo::Stay,
            Err(e) => BrowserGo::Error(e),
        }
    }
}

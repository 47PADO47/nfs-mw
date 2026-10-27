//! The options mods add to a game's settings screens (`bb.settings`), as plain data and pure rules.
//!
//! The game holds an [`OptionsModel`] with the options it has built in, and `add_option` of each mod is applied to it by
//! [`OptionsModel::add_option`]. The rules, exactly:
//!
//! - the option does not exist: it is appended, with the values the mod gave;
//! - it exists with the same kind: the mod's values are appended after the existing ones (a value id that is already
//!   there is skipped, so the game's own value and its position stay), and a mod value comes after the game's own;
//! - it exists with another kind: nothing changes and the caller gets [`AddError::KindMismatch`], which the game logs as
//!   a warning;
//! - it exists but the game fixes its values (a built-in row driven by an enum): nothing is added and the caller gets
//!   [`Merge::Fixed`].
//!
//! A list option with one value is hidden from the menu, and one with more than one is shown ([`OptionDef::visible`]).
//! The game's own rows that mods cannot extend are declared as fixed definitions only so that a clash of kinds is seen.

use std::collections::HashSet;

/// The kind of an option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionKind {
    /// On or off.
    Toggle,
    /// One of several values.
    List,
}

/// The most values one `add_option` may carry, and the most a list may hold.
pub const MAX_OPTION_VALUES: usize = 64;

/// One value of a list option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionValue {
    /// The value's id: what is saved and what the host and the mods compare.
    pub id: String,
    /// The text the menu shows, or a message id of the owner's catalogue.
    pub label: String,
    /// The mod that added the value, or `None` for the game's own.
    pub owner: Option<String>,
}

/// An option of a settings screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionDef {
    /// The option's id.
    pub id: String,
    pub kind: OptionKind,
    /// The row's text, or a message id.
    pub label: String,
    /// A list's values, in the order the menu steps through them. Empty for a toggle.
    pub values: Vec<OptionValue>,
    /// The mod that created the option, or `None` when the game has it.
    pub owner: Option<String>,
    /// A toggle's starting state.
    pub default_on: bool,
    /// The game draws this row itself from its own data; mods cannot add values to it and it is not listed for drawing.
    pub fixed: bool,
}

impl OptionDef {
    /// A list the game has, with its own values (`(id, label)`), which mods may extend.
    pub fn game_list(id: &str, label: &str, values: &[(&str, &str)]) -> Self {
        let values = values
            .iter()
            .map(|(id, label)| OptionValue { id: (*id).to_owned(), label: (*label).to_owned(), owner: None })
            .collect();
        Self {
            id: id.to_owned(),
            kind: OptionKind::List,
            label: label.to_owned(),
            values,
            owner: None,
            default_on: false,
            fixed: false,
        }
    }

    /// A row the game draws itself, known here only by its id and kind.
    pub fn fixed(id: &str, kind: OptionKind) -> Self {
        Self {
            id: id.to_owned(),
            kind,
            label: String::new(),
            values: Vec::new(),
            owner: None,
            default_on: false,
            fixed: true,
        }
    }

    /// Whether the menu shows the option: a toggle always, a list when it has more than one value. A list with one
    /// value has nothing to choose, so it is hidden. A fixed row is drawn by the game, not through the model.
    pub fn visible(&self) -> bool {
        match (self.fixed, self.kind) {
            (true, _) => false,
            (false, OptionKind::Toggle) => true,
            (false, OptionKind::List) => self.values.len() > 1,
        }
    }

    /// The value the option has when nothing is saved: the first one of a list, `off`/`on` for a toggle.
    pub fn default_value(&self) -> String {
        match self.kind {
            OptionKind::Toggle => toggle_text(self.default_on).to_owned(),
            OptionKind::List => self.values.first().map(|value| value.id.clone()).unwrap_or_default(),
        }
    }

    /// The value in effect for a `saved` one: the saved value if the option has it, else the default. A saved value of
    /// a mod that is not loaded any more falls back to the first value.
    pub fn resolve(&self, saved: Option<&str>) -> String {
        let Some(saved) = saved else { return self.default_value() };
        match self.kind {
            OptionKind::Toggle => match saved {
                "on" | "off" => saved.to_owned(),
                _ => self.default_value(),
            },
            OptionKind::List => match self.values.iter().any(|value| value.id == saved) {
                true => saved.to_owned(),
                false => self.default_value(),
            },
        }
    }

    /// The value one step after (`forward`) or before `current`, wrapping round. A value that is not there steps from the
    /// first. A list with no values answers an empty text.
    pub fn step(&self, current: &str, forward: bool) -> String {
        if self.kind == OptionKind::Toggle {
            return toggle_text(current != "on").to_owned();
        }
        let len = self.values.len();
        if len == 0 {
            return String::new();
        }
        let at = self.values.iter().position(|value| value.id == current).unwrap_or(0);
        let next = match forward {
            true => (at + 1) % len,
            false => (at + len - 1) % len,
        };
        self.values[next].id.clone()
    }
}

/// `"on"` or `"off"`.
pub fn toggle_text(on: bool) -> &'static str {
    match on {
        true => "on",
        false => "off",
    }
}

/// A value the mod gives, borrowed for the call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueSpec<'a> {
    pub id: &'a str,
    pub label: &'a str,
}

/// What a mod gives to `add_option`, borrowed for the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionSpec<'a> {
    pub id: &'a str,
    pub kind: OptionKind,
    pub label: &'a str,
    pub values: Vec<ValueSpec<'a>>,
    pub default_on: bool,
}

/// What an `add_option` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merge {
    /// The option did not exist and was appended.
    Created,
    /// The option existed with the same kind: `added` are the value ids appended, `skipped` how many the option had
    /// already.
    Integrated { added: Vec<String>, skipped: usize },
    /// The option is a built-in row whose values the game fixes: nothing was added.
    Fixed,
}

/// Why an `add_option` changed nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddError {
    /// The option exists with another kind. The game warns and does nothing.
    KindMismatch { existing: OptionKind },
    /// The spec is wrong: the text says why.
    Invalid(&'static str),
}

/// What one `add_option` left behind, so that it can be taken back.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Registration {
    handle: u64,
    owner: String,
    option: String,
    added: Vec<String>,
    created: bool,
}

/// The options of a settings screen: the game's own, and what the mods added.
#[derive(Debug, Clone, Default)]
pub struct OptionsModel {
    options: Vec<OptionDef>,
    registrations: Vec<Registration>,
    last_handle: u64,
}

/// Whether `text` can be an id: not empty, at most 128 bytes, and only `a-z`, `0-9`, `_`, `.` and `-`.
pub fn is_plain_id(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 128
        && text.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-'))
}

impl OptionsModel {
    /// A model holding the options the game has.
    pub fn new(builtin: Vec<OptionDef>) -> Self {
        Self { options: builtin, registrations: Vec::new(), last_handle: 0 }
    }

    /// Every option, in order: the game's, then the mods'.
    pub fn options(&self) -> &[OptionDef] {
        &self.options
    }

    /// The option with this id.
    pub fn get(&self, id: &str) -> Option<&OptionDef> {
        self.options.iter().find(|option| option.id == id)
    }

    /// The options the menu draws, in order: those that are [visible](OptionDef::visible).
    pub fn visible(&self) -> Vec<&OptionDef> {
        self.options.iter().filter(|option| option.visible()).collect()
    }

    /// Apply a mod's `add_option`. On success, the registration's handle and what happened.
    pub fn add_option(&mut self, owner: &str, spec: &OptionSpec<'_>) -> Result<(u64, Merge), AddError> {
        check_spec(spec)?;
        let merge = match self.options.iter().position(|option| option.id == spec.id) {
            None => self.create(owner, spec),
            Some(at) => self.integrate(owner, at, spec)?,
        };
        self.last_handle += 1;
        let handle = self.last_handle;
        let (added, created) = match &merge {
            Merge::Created => (spec.values.iter().map(|value| value.id.to_owned()).collect(), true),
            Merge::Integrated { added, .. } => (added.clone(), false),
            Merge::Fixed => (Vec::new(), false),
        };
        self.registrations.push(Registration {
            handle,
            owner: owner.to_owned(),
            option: spec.id.to_owned(),
            added,
            created,
        });
        Ok((handle, merge))
    }

    fn create(&mut self, owner: &str, spec: &OptionSpec<'_>) -> Merge {
        let values = spec
            .values
            .iter()
            .map(|value| OptionValue {
                id: value.id.to_owned(),
                label: value.label.to_owned(),
                owner: Some(owner.to_owned()),
            })
            .collect();
        self.options.push(OptionDef {
            id: spec.id.to_owned(),
            kind: spec.kind,
            label: spec.label.to_owned(),
            values,
            owner: Some(owner.to_owned()),
            default_on: spec.default_on,
            fixed: false,
        });
        Merge::Created
    }

    fn integrate(&mut self, owner: &str, at: usize, spec: &OptionSpec<'_>) -> Result<Merge, AddError> {
        let option = &mut self.options[at];
        if option.kind != spec.kind {
            return Err(AddError::KindMismatch { existing: option.kind });
        }
        if option.fixed {
            return Ok(Merge::Fixed);
        }
        let (mut added, mut skipped) = (Vec::new(), 0);
        for value in &spec.values {
            if option.values.iter().any(|known| known.id == value.id) {
                skipped += 1;
                continue;
            }
            if option.values.len() >= MAX_OPTION_VALUES {
                return Err(AddError::Invalid("the option already has the most values a list may hold"));
            }
            option.values.push(OptionValue {
                id: value.id.to_owned(),
                label: value.label.to_owned(),
                owner: Some(owner.to_owned()),
            });
            added.push(value.id.to_owned());
        }
        Ok(Merge::Integrated { added, skipped })
    }

    /// Take back one registration of `owner`: the values it appended go, and so does the option if the mod created it and
    /// nothing is left in it. Returns whether the registration was there.
    pub fn remove(&mut self, owner: &str, handle: u64) -> bool {
        let Some(at) = self
            .registrations
            .iter()
            .position(|registration| registration.handle == handle && registration.owner == owner)
        else {
            return false;
        };
        let registration = self.registrations.remove(at);
        let Some(option) = self.options.iter_mut().find(|option| option.id == registration.option) else {
            return true;
        };
        option
            .values
            .retain(|value| !(value.owner.as_deref() == Some(owner) && registration.added.contains(&value.id)));
        let empty_list = option.kind == OptionKind::List && option.values.is_empty();
        let gone = registration.created && (option.kind == OptionKind::Toggle || empty_list);
        if gone {
            self.options.retain(|option| option.id != registration.option);
        }
        true
    }

    /// Take back every registration of `owner`, as when the mod unloads.
    pub fn remove_owner(&mut self, owner: &str) {
        let handles: Vec<u64> = self
            .registrations
            .iter()
            .filter(|registration| registration.owner == owner)
            .map(|registration| registration.handle)
            .collect();
        for handle in handles {
            self.remove(owner, handle);
        }
    }

    /// The mod that provides value `value` of option `option`, if it is a mod's.
    pub fn value_owner(&self, option: &str, value: &str) -> Option<&str> {
        self.get(option)?.values.iter().find(|known| known.id == value)?.owner.as_deref()
    }
}

fn check_spec(spec: &OptionSpec<'_>) -> Result<(), AddError> {
    if !is_plain_id(spec.id) {
        return Err(AddError::Invalid("the option id must be plain: a-z, 0-9, _, . and -"));
    }
    if spec.values.len() > MAX_OPTION_VALUES {
        return Err(AddError::Invalid("too many values"));
    }
    match spec.kind {
        OptionKind::Toggle if !spec.values.is_empty() => {
            return Err(AddError::Invalid("a toggle has no values"));
        }
        OptionKind::List if spec.values.is_empty() => {
            return Err(AddError::Invalid("a list needs at least one value"));
        }
        _ => {}
    }
    let mut seen = HashSet::new();
    for value in &spec.values {
        if !is_plain_id(value.id) {
            return Err(AddError::Invalid("a value id must be plain: a-z, 0-9, _, . and -"));
        }
        if value.label.is_empty() {
            return Err(AddError::Invalid("a value needs a label"));
        }
        if !seen.insert(value.id) {
            return Err(AddError::Invalid("a value id is given twice"));
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;

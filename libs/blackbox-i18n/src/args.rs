//! The variables a message is formatted with, owned so that a text can be kept and formatted again after a language change.

use fluent_bundle::{FluentArgs, FluentValue};

/// One variable's value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Arg {
    /// A whole number: it selects plural and ordinal forms.
    Int(i64),
    /// Text shown as it is.
    Str(String),
}

/// The named variables of a message, in the order they were set.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Args(Vec<(String, Arg)>);

impl Args {
    pub fn new() -> Self {
        Self::default()
    }

    /// These arguments with `name` set to `value` (replacing an earlier value of the same name).
    pub fn with(mut self, name: &str, value: impl Into<Arg>) -> Self {
        self.set(name, value);
        self
    }

    pub fn set(&mut self, name: &str, value: impl Into<Arg>) {
        let value = value.into();
        if let Some(slot) = self.0.iter_mut().find(|(n, _)| n == name) {
            slot.1 = value;
            return;
        }
        self.0.push((name.to_owned(), value));
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The names that are set.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(name, _)| name.as_str())
    }

    pub(crate) fn to_fluent(&self) -> FluentArgs<'_> {
        let mut out = FluentArgs::with_capacity(self.0.len());
        for (name, value) in &self.0 {
            let value = match value {
                Arg::Int(n) => FluentValue::from(*n),
                Arg::Str(s) => FluentValue::from(s.as_str()),
            };
            out.set(name.as_str(), value);
        }
        out
    }
}

macro_rules! int_arg {
    ($($t:ty),*) => {$(
        impl From<$t> for Arg {
            fn from(value: $t) -> Self {
                Self::Int(i64::try_from(value).unwrap_or(i64::MAX))
            }
        }
    )*};
}
int_arg!(u64, usize, isize);

macro_rules! small_int_arg {
    ($($t:ty),*) => {$(
        impl From<$t> for Arg {
            fn from(value: $t) -> Self {
                Self::Int(i64::from(value))
            }
        }
    )*};
}
small_int_arg!(i8, i16, i32, u8, u16, u32);

impl From<i64> for Arg {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<String> for Arg {
    fn from(value: String) -> Self {
        Self::Str(value)
    }
}

impl From<&str> for Arg {
    fn from(value: &str) -> Self {
        Self::Str(value.to_owned())
    }
}

impl From<&String> for Arg {
    fn from(value: &String) -> Self {
        Self::Str(value.clone())
    }
}

//! A text value a page lets the player type: the room name, the password, an address, a chat line.

/// A typed key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typed {
    Char(char),
    Backspace,
}

/// A text value, with how much it holds and whether it is shown as dots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub text: String,
    /// Most characters.
    pub max: usize,
    /// Shown as dots (a password).
    pub secret: bool,
    /// Keys go to the field.
    pub editing: bool,
}

impl Field {
    pub fn new(text: &str, max: usize) -> Self {
        Self { text: text.chars().take(max).collect(), max, secret: false, editing: false }
    }

    pub fn secret(max: usize) -> Self {
        Self { secret: true, ..Self::new("", max) }
    }

    /// Takes a typed key while editing. Control characters are left out.
    pub fn take(&mut self, key: Typed) {
        if !self.editing {
            return;
        }
        match key {
            Typed::Backspace => {
                self.text.pop();
            }
            Typed::Char(c) if !c.is_control() && self.text.chars().count() < self.max => self.text.push(c),
            Typed::Char(_) => {}
        }
    }

    /// The value as the row shows it: dots for a secret, and a caret while editing.
    pub fn shown(&self) -> String {
        let mut text = match self.secret {
            true => "*".repeat(self.text.chars().count()),
            false => self.text.clone(),
        };
        if self.editing {
            text.push('_');
        }
        text
    }

    /// The text without the spaces around it.
    pub fn value(&self) -> &str {
        self.text.trim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_takes_keys_only_while_editing_and_up_to_its_size() {
        let mut field = Field::new("ab", 3);
        field.take(Typed::Char('c'));
        assert_eq!(field.text, "ab");
        field.editing = true;
        for c in "cde".chars() {
            field.take(Typed::Char(c));
        }
        field.take(Typed::Char('\n'));
        assert_eq!(field.shown(), "abc_");
        field.take(Typed::Backspace);
        assert_eq!(field.text, "ab");
        let mut secret = Field::secret(8);
        secret.editing = true;
        secret.take(Typed::Char('p'));
        secret.take(Typed::Char('w'));
        secret.editing = false;
        assert_eq!((secret.shown(), secret.value()), ("**".to_owned(), "pw"));
    }
}

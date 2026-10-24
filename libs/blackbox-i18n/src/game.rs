//! The game-label side: a game's language table, or a language pack's translations of its labels.

use std::collections::HashMap;

use blackbox_text::{StringTable, upper_label_hash};
use fluent_syntax::ast::{Entry, Expression, InlineExpression, PatternElement};

use crate::{Locale, Problem};

enum Table {
    /// A `LANGUAGES/*.bin` table.
    Bin(StringTable),
    /// Labels from `.ftl` files, by the upper-case hash of their names.
    Pack(HashMap<u32, String>),
}

/// The text of game labels in one locale.
pub struct GameLayer {
    pub locale: Locale,
    /// Who supplied it, for messages (`English.bin`, a mod id).
    pub origin: String,
    table: Table,
}

impl GameLayer {
    /// A game's own table.
    pub fn table(locale: Locale, origin: &str, table: StringTable) -> Self {
        Self { locale, origin: origin.to_owned(), table: Table::Bin(table) }
    }

    /// Labels a language pack writes as Fluent messages: the message id is the label name, the value is plain text with the
    /// game's printf places (`%d`, `%s`). A literal placeable such as `{"{"}` is its text; any other placeable is a problem
    /// and is left out.
    pub fn pack(locale: Locale, origin: &str, files: &[(String, String)]) -> (Self, Vec<Problem>) {
        let mut labels = HashMap::new();
        let mut problems = Vec::new();
        for (file, text) in files {
            let problem = |message: String| Problem { origin: origin.to_owned(), file: file.clone(), message };
            let resource = match fluent_syntax::parser::parse(text.as_str()) {
                Ok(resource) => resource,
                Err((resource, errors)) => {
                    problems.extend(errors.iter().map(|e| problem(format!("{e:?}"))));
                    resource
                }
            };
            for entry in resource.body {
                let Entry::Message(message) = entry else { continue };
                let Some(pattern) = message.value else { continue };
                let mut value = String::new();
                for element in pattern.elements {
                    match element {
                        PatternElement::TextElement { value: text } => value.push_str(text),
                        PatternElement::Placeable {
                            expression: Expression::Inline(InlineExpression::StringLiteral { value: text }),
                        } => value.push_str(text),
                        PatternElement::Placeable { .. } => {
                            problems.push(problem(format!("{}: game labels take plain text", message.id.name)));
                        }
                    }
                }
                labels.insert(upper_label_hash(message.id.name), value);
            }
        }
        (Self { locale, origin: origin.to_owned(), table: Table::Pack(labels) }, problems)
    }

    /// The text of `label`, if this layer has it.
    pub fn get(&self, label: u32) -> Option<String> {
        match &self.table {
            Table::Bin(table) => table.get(label),
            Table::Pack(labels) => labels.get(&label).cloned(),
        }
    }

    /// Whether this is a game's own table rather than a pack.
    pub fn is_table(&self) -> bool {
        matches!(self.table, Table::Bin(_))
    }
}

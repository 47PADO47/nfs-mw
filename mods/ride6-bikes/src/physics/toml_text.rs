//! A small writer of the TOML text of the `[bike]` table. Numbers are written as `f32` with the shortest text that reads back
//! as the same `f32` (what `{:?}` prints), so the manifest holds `0.45` and not `0.44999998`.

use std::fmt::Write;

/// The text under construction, and the notes the conversion wants the user to read (a choice it had to make, a field of the
/// data that has no key in the manifest).
#[derive(Default)]
pub struct Toml {
    text: String,
    notes: Vec<String>,
}

fn number(value: f64) -> String {
    format!("{:?}", value as f32)
}

impl Toml {
    pub fn new() -> Self {
        Self::default()
    }

    /// A `[name]` header, after a blank line.
    pub fn table(&mut self, name: &str) {
        if !self.text.is_empty() {
            self.text.push('\n');
        }
        let _ = writeln!(self.text, "[{name}]");
    }

    /// A `[[name]]` header: a new element of an array of tables.
    pub fn table_array(&mut self, name: &str) {
        let _ = writeln!(self.text, "\n[[{name}]]");
    }

    pub fn flag(&mut self, key: &str, value: bool) {
        let _ = writeln!(self.text, "{key} = {value}");
    }

    /// A comment line.
    pub fn comment(&mut self, line: &str) {
        let _ = writeln!(self.text, "# {line}");
    }

    pub fn num(&mut self, key: &str, value: f64) {
        let _ = writeln!(self.text, "{key} = {}", number(value));
    }

    pub fn nums(&mut self, key: &str, values: &[f64]) {
        let items: Vec<String> = values.iter().map(|&v| number(v)).collect();
        let _ = writeln!(self.text, "{key} = [{}]", items.join(", "));
    }

    /// A list of `[x, y]` pairs.
    pub fn pairs(&mut self, key: &str, points: &[[f64; 2]]) {
        let items: Vec<String> = points.iter().map(|p| format!("[{}, {}]", number(p[0]), number(p[1]))).collect();
        let _ = writeln!(self.text, "{key} = [{}]", items.join(", "));
    }

    pub fn note(&mut self, text: impl Into<String>) {
        self.notes.push(text.into());
    }

    pub fn into_string(self) -> String {
        self.text
    }

    /// The text and the notes.
    pub fn into_parts(self) -> (String, Vec<String>) {
        (self.text, self.notes)
    }
}

/// `value` as a TOML basic string.
pub fn quote(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_print_as_the_shortest_f32_text() {
        let mut toml = Toml::new();
        toml.table("a");
        toml.num("x", 0.45);
        toml.nums("y", &[1.0, 2.5]);
        toml.pairs("z", &[[0.0, 1.0], [1e-6, 2.0]]);
        let text = toml.into_string();
        assert_eq!(text, "[a]\nx = 0.45\ny = [1.0, 2.5]\nz = [[0.0, 1.0], [1e-6, 2.0]]\n");
    }

    #[test]
    fn strings_are_escaped() {
        assert_eq!(quote("KTM \"450\" \\ x"), "\"KTM \\\"450\\\" \\\\ x\"");
    }
}

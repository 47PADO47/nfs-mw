//! Sections: rows grouped under a title. The first section is the default one and has no title, so a page that has
//! no sections looks as it always did; a section with no rows shows no title.

/// A title row (when it has one) and the rows under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section<R> {
    pub title: Option<R>,
    pub rows: Vec<R>,
}

impl<R> Section<R> {
    /// The default section: rows and no title.
    pub fn untitled(rows: Vec<R>) -> Self {
        Self { title: None, rows }
    }

    /// A section under `title`.
    pub fn titled(title: R, rows: Vec<R>) -> Self {
        Self { title: Some(title), rows }
    }

    /// A section with no rows shows nothing, not even its title.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// The rows of `sections` in order, each titled section's title first. A section with no rows adds nothing.
pub fn flatten<R>(sections: impl IntoIterator<Item = Section<R>>) -> Vec<R> {
    let mut out = Vec::new();
    for section in sections {
        if section.is_empty() {
            continue;
        }
        out.extend(section.title);
        out.extend(section.rows);
    }
    out
}

/// Sections being collected by a key: the default section (no key) first, then one section per distinct key in the order
/// each was first used. Rows with no key go to the default section, as they do on a page that has no sections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionSet<R> {
    default: Vec<R>,
    titled: Vec<(String, R, Vec<R>)>,
}

impl<R> Default for SectionSet<R> {
    fn default() -> Self {
        Self { default: Vec::new(), titled: Vec::new() }
    }
}

impl<R> SectionSet<R> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `row` to the default section.
    pub fn push_default(&mut self, row: R) {
        self.default.push(row);
    }

    /// Fixes the place of the section `key` among the titled ones before any row is added to it: sections come out in the
    /// order they were declared or first used. A section declared and never used has no rows, so it shows no title.
    pub fn declare(&mut self, key: &str, title: R) {
        if key.is_empty() || self.titled.iter().any(|(known, _, _)| known == key) {
            return;
        }
        self.titled.push((key.to_owned(), title, Vec::new()));
    }

    /// Adds `row` to the section `key`. `title` makes the title row when the section is new. An empty key is the default
    /// section.
    pub fn push(&mut self, key: &str, title: impl FnOnce() -> R, row: R) {
        if key.is_empty() {
            return self.default.push(row);
        }
        if let Some((_, _, rows)) = self.titled.iter_mut().find(|(known, _, _)| known == key) {
            return rows.push(row);
        }
        self.titled.push((key.to_owned(), title(), vec![row]));
    }

    /// The sections in order: the default one, then the titled ones.
    pub fn sections(self) -> Vec<Section<R>> {
        let titled = self.titled.into_iter().map(|(_, title, rows)| Section::titled(title, rows));
        std::iter::once(Section::untitled(self.default)).chain(titled).collect()
    }

    /// The flat list, titles included.
    pub fn into_rows(self) -> Vec<R> {
        flatten(self.sections())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_section_has_no_title_and_keeps_its_rows_first() {
        let mut set = SectionSet::new();
        set.push_default("a");
        set.push("Mod", || "[Mod]", "m1");
        set.push_default("b");
        assert_eq!(set.into_rows(), ["a", "b", "[Mod]", "m1"]);
    }

    #[test]
    fn a_key_used_twice_is_one_section_in_the_order_it_first_appeared() {
        let mut set = SectionSet::new();
        set.push("One", || "[One]", "x");
        set.push("Two", || "[Two]", "y");
        set.push("One", || "never made", "z");
        assert_eq!(set.into_rows(), ["[One]", "x", "z", "[Two]", "y"]);
    }

    #[test]
    fn a_declared_section_keeps_its_place_and_shows_no_title_until_it_has_rows() {
        let mut set = SectionSet::new();
        set.declare("First", "[First]");
        set.declare("Second", "[Second]");
        set.push("Second", || "never made", "s");
        set.push_default("d");
        assert_eq!(set.clone().into_rows(), ["d", "[Second]", "s"], "First has no rows");
        set.push("First", || "never made", "f");
        assert_eq!(set.into_rows(), ["d", "[First]", "f", "[Second]", "s"]);
    }

    #[test]
    fn an_empty_key_is_the_default_section() {
        let mut set = SectionSet::new();
        set.push("", || "[never]", "d");
        assert_eq!(set.into_rows(), ["d"]);
    }

    #[test]
    fn a_section_with_no_rows_shows_no_title() {
        let sections = vec![Section::untitled(vec![1, 2]), Section::titled(100, vec![]), Section::titled(200, vec![3])];
        assert_eq!(flatten(sections), [1, 2, 200, 3]);
        assert_eq!(flatten([Section::titled(1, Vec::<i32>::new())]), Vec::<i32>::new());
        assert!(SectionSet::<u8>::new().into_rows().is_empty());
    }
}

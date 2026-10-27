use crate::id::ResourceId;

/// One `<Resource .../>` row of a bundle's `ResourceStringTable`: the debug name of a resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugName {
    /// The id as written: 16 hex digits are the full id, 8 digits are a bare key (class, sub-type and index zero).
    pub id: ResourceId,
    /// The type name the table gives (`Texture`, `Material`, ...).
    pub kind: String,
    pub name: String,
    /// The stream, when the row has a numeric `streamIndex` attribute.
    pub stream: Option<u8>,
}

/// Read the rows of a `ResourceStringTable`. A row without an `id`, `type` or `name` attribute, or with an id that is
/// not 8 or 16 hex digits, is skipped.
pub fn parse_debug_names(xml: &str) -> Vec<DebugName> {
    let mut names = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find("<Resource ") {
        rest = &rest[at + "<Resource ".len()..];
        let end = rest.find('>').unwrap_or(rest.len());
        if let Some(row) = row_from(&rest[..end]) {
            names.push(row);
        }
        rest = &rest[end..];
    }
    names
}

fn row_from(tag: &str) -> Option<DebugName> {
    let id = attribute(tag, "id")?;
    if id.len() != 8 && id.len() != 16 {
        return None;
    }
    Some(DebugName {
        id: ResourceId(u64::from_str_radix(&id, 16).ok()?),
        kind: attribute(tag, "type")?,
        name: attribute(tag, "name")?,
        stream: attribute(tag, "streamIndex").and_then(|value| value.parse().ok()),
    })
}

/// The unescaped value of attribute `key` in a tag body; the key must start at a word boundary.
fn attribute(tag: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=\"");
    let mut from = 0;
    while let Some(found) = tag[from..].find(&needle) {
        let start = from + found;
        let at_boundary = start == 0 || tag.as_bytes()[start - 1].is_ascii_whitespace();
        let value = start + needle.len();
        if at_boundary {
            let len = tag[value..].find('"')?;
            return Some(unescape(&tag[value..value + len]));
        }
        from = value;
    }
    None
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

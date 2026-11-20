//! The surfaces that took the material of a similar model: `fallbacks[]` of `parts.json` (written by the exporter's `fallbacks.py`)
//! and the comment lines that tell the manifest's reader where each picture came from
//! ([the rules](../../../docs/modding/ride6-bikes-batch.md#grey-surfaces-the-fallbacks)).
//!
//! The material and its pictures are already in the part table and the bike's `tex/` folder, so the importer builds the surface like any
//! other; this module only keeps the record, so a manifest says which picture of which bike a surface wears and by which rule.

use serde_json::Value;

/// One surface that no picture reached and that wears another model's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fallback {
    /// The part (`front_rim`, `homologation`, `chassis`) and the name of its surface in the exported mesh.
    pub part: String,
    pub surface: String,
    /// The material and picture it wears, and the bike they were taken from (`None`: a flat colour made for it).
    pub material: String,
    pub picture: String,
    pub source: Option<String>,
    /// The rule that chose it (`asset`, `family`, `shape`, `own`, `make`, `flat`).
    pub rule: String,
    /// The triangles of the surface at the finest level of detail (0 when it is only on the far levels).
    pub triangles: u64,
}

impl Fallback {
    /// The record of one `fallbacks[]` entry, or `None` when it lacks the fields.
    pub fn read(value: &Value) -> Option<Self> {
        let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_owned);
        let at_finest =
            value.get("levels").and_then(Value::as_array).is_some_and(|levels| levels.iter().any(|l| l == 0));
        Some(Self {
            part: text("part")?,
            surface: text("surface")?,
            material: text("material")?,
            picture: text("picture")?,
            source: text("source"),
            rule: text("rule")?,
            triangles: match at_finest {
                true => value.get("triangles").and_then(Value::as_u64).unwrap_or(0),
                false => 0,
            },
        })
    }

    /// The manifest comment: `# fallback: <part> <surface> (<triangles> triangles at LOD0) wears <picture> of <bike> [rule]`.
    pub fn comment(&self) -> String {
        let from = self
            .source
            .as_deref()
            .map_or_else(|| "a flat colour".to_owned(), |bike| format!("{} of {bike}", self.material));
        format!(
            "# fallback: {} {} ({} triangles at LOD0) wears {} ({from}) [{}]",
            self.part, self.surface, self.triangles, self.picture, self.rule
        )
    }
}

/// The `fallbacks[]` of a part table document, in the file's order; none for a file without them.
pub fn read_all(doc: &Value) -> Vec<Fallback> {
    doc.get("fallbacks")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Fallback::read).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Value {
        serde_json::json!({
            "fallbacks": [
                {"part": "front_rim", "surface": "mat_rim", "levels": [0, 1], "triangles": 8717, "rule": "family",
                 "source": "Ducati_Monster-1200-S-RM-2019", "material": "mat_rim718_001", "picture": "rim718_001_d"},
                {"part": "chassis", "surface": "Default_Material", "levels": [1], "triangles": 228, "rule": "flat",
                 "source": null, "material": "flat", "picture": "808080"},
                {"part": "broken"}
            ]
        })
    }

    #[test]
    fn the_records_are_read_in_order_and_an_incomplete_one_is_skipped() {
        let all = read_all(&sample());
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].surface, "mat_rim");
        assert_eq!(all[0].triangles, 8717);
        assert_eq!(all[1].source, None);
    }

    #[test]
    fn a_surface_only_on_the_far_levels_counts_no_triangles_at_lod0() {
        assert_eq!(read_all(&sample())[1].triangles, 0);
    }

    #[test]
    fn the_comment_names_the_picture_the_bike_and_the_rule() {
        let all = read_all(&sample());
        assert_eq!(
            all[0].comment(),
            "# fallback: front_rim mat_rim (8717 triangles at LOD0) wears rim718_001_d (mat_rim718_001 of Ducati_Monster-1200-S-RM-2019) [family]"
        );
        assert_eq!(
            all[1].comment(),
            "# fallback: chassis Default_Material (0 triangles at LOD0) wears 808080 (a flat colour) [flat]"
        );
    }

    #[test]
    fn a_document_without_fallbacks_has_none() {
        assert!(read_all(&serde_json::json!({"schema": 1})).is_empty());
    }
}
